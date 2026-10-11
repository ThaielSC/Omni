use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, BufReader, Stdout, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use zeroize::{Zeroize, Zeroizing};

use crate::config::{
    Action, OmniConfig, ProjectManagersConfig, ThemeColors, default_config_path,
    default_keymap_path, default_managers_path, default_theme_path, parse_keymap_toml,
    parse_managers_toml, parse_theme_toml,
};
use crate::model::{FilterTab, InstallReason, SortMode, UnifiedPackage};
use crate::provider::{
    PackageManagerProvider, build_project_add_command, build_uninstall_command_with_projects,
    build_update_command,
    cargo::CargoProvider,
    custom::CustomManager,
    flatpak::FlatpakProvider,
    pacman::PacmanProvider,
    pip::PipProvider,
    project::{ProjectContext, discover_projects},
    uv::UvProvider,
    yay::YayProvider,
};
use crate::utils::format::format_bytes;

use super::{help, layout, statusline};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Managers,
    Packages,
    PackageDependencies,
    Details,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Overlay {
    Help,
    ConfirmUninstall,
    ConfirmUpdate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperationKind {
    Uninstall,
    Update,
    Add,
}

impl OperationKind {
    fn label(self) -> &'static str {
        match self {
            Self::Uninstall => "UNINSTALL",
            Self::Update => "UPDATE",
            Self::Add => "ADD",
        }
    }
}

#[derive(Debug)]
enum UiEvent {
    Packages((Vec<UnifiedPackage>, Vec<String>, Vec<ProjectContext>)),
    OperationOutput(String),
    OperationFinished(Option<i32>),
    Updates(u64, HashMap<PackageIdentity, String>, Vec<String>),
}

type PackageIdentity = (crate::model::PackageSource, Option<String>, String);

fn package_identity(package: &UnifiedPackage) -> PackageIdentity {
    (
        package.source,
        package.manager_id.clone(),
        package.name.clone(),
    )
}

#[derive(Debug, Clone)]
struct UninstallConsole {
    operation: OperationKind,
    package: String,
    lines: Vec<String>,
    running: bool,
    exit_code: Option<i32>,
}

struct App {
    config: OmniConfig,
    packages: Vec<UnifiedPackage>,
    filtered: Vec<usize>,
    tab: FilterTab,
    focus: Focus,
    manager_index: usize,
    project_managers: Vec<ProjectContext>,
    active_project_manager: Option<String>,
    package_index: usize,
    detail_scroll: u16,
    sort: SortMode,
    show_dependencies: bool,
    query: String,
    searching: bool,
    project_add_input: Option<String>,
    overlay: Option<Overlay>,
    help_filter: String,
    help_filtering: bool,
    loading: bool,
    checking_updates: bool,
    started_at: Instant,
    reload_after_loading: bool,
    notice: Option<(String, Instant)>,
    notice_queue: Vec<String>,
    reload_tx: Sender<UiEvent>,
    show_package_dependencies: bool,
    dependency_index: usize,
    uninstall_targets: Vec<UnifiedPackage>,
    uninstall_commands: Vec<crate::provider::UninstallCommand>,
    uninstall_console: Option<UninstallConsole>,
    operation_kind: OperationKind,
    console_scroll: usize,
    sudo_password_input: Option<Zeroizing<Vec<u8>>>,
    selected_packages: HashSet<PackageIdentity>,
    updates_only: bool,
    update_versions: HashMap<PackageIdentity, String>,
    updates_checked: bool,
    inventory_generation: u64,
}

impl App {
    fn new(config: OmniConfig, reload_tx: Sender<UiEvent>) -> Self {
        Self {
            config,
            packages: Vec::new(),
            filtered: Vec::new(),
            tab: FilterTab::All,
            focus: Focus::Packages,
            manager_index: 0,
            project_managers: Vec::new(),
            active_project_manager: None,
            package_index: 0,
            detail_scroll: 0,
            sort: SortMode::SizeDesc,
            show_dependencies: false,
            query: String::new(),
            searching: false,
            project_add_input: None,
            overlay: None,
            help_filter: String::new(),
            help_filtering: false,
            loading: true,
            checking_updates: false,
            started_at: Instant::now(),
            reload_after_loading: false,
            notice: None,
            notice_queue: Vec::new(),
            reload_tx,
            show_package_dependencies: false,
            dependency_index: 0,
            uninstall_targets: Vec::new(),
            uninstall_commands: Vec::new(),
            uninstall_console: None,
            operation_kind: OperationKind::Uninstall,
            console_scroll: 0,
            sudo_password_input: None,
            selected_packages: HashSet::new(),
            updates_only: false,
            update_versions: HashMap::new(),
            updates_checked: false,
            inventory_generation: 0,
        }
    }

    fn update_filter(&mut self) {
        let matcher = SkimMatcherV2::default();
        self.filtered = self
            .packages
            .iter()
            .enumerate()
            .filter(|(_, package)| {
                if let Some(manager_id) = &self.active_project_manager {
                    package.manager_id.as_ref() == Some(manager_id)
                } else {
                    self.tab.matches(package.source)
                }
            })
            .filter(|(_, package)| {
                self.show_dependencies || package.install_reason != InstallReason::Dependency
            })
            .filter(|(_, package)| {
                !self.updates_only
                    || self
                        .update_versions
                        .contains_key(&package_identity(package))
            })
            .filter(|(_, package)| {
                if self.query.is_empty() {
                    return true;
                }
                matcher.fuzzy_match(&package.name, &self.query).is_some()
                    || package
                        .description
                        .as_deref()
                        .is_some_and(|desc| matcher.fuzzy_match(desc, &self.query).is_some())
            })
            .map(|(index, _)| index)
            .collect();
        self.package_index = self
            .package_index
            .min(self.filtered.len().saturating_sub(1));
        self.detail_scroll = 0;
    }

    fn selected_package(&self) -> Option<&UnifiedPackage> {
        self.filtered
            .get(self.package_index)
            .and_then(|index| self.packages.get(*index))
    }

    fn selected_dependency_name(&self) -> Option<&str> {
        self.selected_package()?
            .dependencies
            .get(self.dependency_index)
            .map(String::as_str)
    }

    fn selected_dependency_package(&self) -> Option<&UnifiedPackage> {
        let parent = self.selected_package()?;
        let name = dependency_package_name(self.selected_dependency_name()?);
        self.packages.iter().find(|package| {
            package.name == name
                && package.source == parent.source
                && package.manager_id == parent.manager_id
        })
    }

    fn selected_detail_package(&self) -> Option<&UnifiedPackage> {
        if self.show_package_dependencies
            && self
                .selected_package()
                .is_some_and(|p| !p.dependencies.is_empty())
        {
            self.selected_dependency_package()
        } else {
            self.selected_package()
        }
    }

    fn manager_counts(&self) -> [usize; 6] {
        let matcher = SkimMatcherV2::default();
        FilterTab::ALL.map(|tab| {
            self.packages
                .iter()
                .filter(|p| tab.matches(p.source))
                .filter(|p| self.show_dependencies || p.install_reason != InstallReason::Dependency)
                .filter(|p| {
                    self.query.is_empty()
                        || matcher.fuzzy_match(&p.name, &self.query).is_some()
                        || p.description
                            .as_deref()
                            .is_some_and(|d| matcher.fuzzy_match(d, &self.query).is_some())
                })
                .count()
        })
    }

    fn active_project(&self) -> Option<&ProjectContext> {
        let id = self.active_project_manager.as_ref()?;
        self.project_managers
            .iter()
            .find(|project| &project.manager_id == id)
    }

    fn select_manager_index(&mut self, index: usize) {
        self.manager_index = index;
        if let Some(tab) = FilterTab::from_index(index) {
            self.active_project_manager = None;
            self.tab = tab;
        } else if let Some(project) = self
            .project_managers
            .get(index.saturating_sub(FilterTab::ALL.len()))
        {
            self.active_project_manager = Some(project.manager_id.clone());
        }
        self.package_index = 0;
        self.update_filter();
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.project_add_input.is_some() {
            match key.code {
                KeyCode::Esc => self.project_add_input = None,
                KeyCode::Enter => {
                    let name = self.project_add_input.take().unwrap_or_default();
                    self.start_project_add(name);
                }
                KeyCode::Backspace => {
                    if let Some(input) = self.project_add_input.as_mut() {
                        input.pop();
                    }
                }
                KeyCode::Char(character)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    if let Some(input) = self.project_add_input.as_mut() {
                        input.push(character);
                    }
                }
                _ => {}
            }
            return false;
        }
        if self.sudo_password_input.is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.sudo_password_input = None;
                    self.uninstall_targets.clear();
                    self.uninstall_commands.clear();
                    self.set_notice(format!(
                        "Password entry cancelled; {} was not run",
                        self.operation_kind.label().to_lowercase()
                    ));
                }
                KeyCode::Enter => {
                    let password = self.sudo_password_input.take().unwrap();
                    self.launch_operation(Some(password));
                }
                KeyCode::Backspace => {
                    let Some(password) = self.sudo_password_input.as_mut() else {
                        return false;
                    };
                    if let Some(index) = password
                        .iter()
                        .rposition(|byte| byte & 0b1100_0000 != 0b1000_0000)
                    {
                        password.truncate(index);
                    }
                }
                KeyCode::Char(character)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    let Some(password) = self.sudo_password_input.as_mut() else {
                        return false;
                    };
                    let mut encoded = [0; 4];
                    password.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
                }
                _ => {}
            }
            return false;
        }

        if self.searching {
            match key.code {
                KeyCode::Esc => {
                    self.searching = false;
                    self.query.clear();
                    self.package_index = 0;
                    self.update_filter();
                }
                KeyCode::Enter => self.searching = false,
                KeyCode::Backspace => {
                    self.query.pop();
                    self.package_index = 0;
                    self.update_filter();
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.query.push(c);
                    self.package_index = 0;
                    self.update_filter();
                }
                _ => {}
            }
            return false;
        }

        if let Some(overlay) = self.overlay {
            if overlay == Overlay::Help && self.help_filtering {
                match key.code {
                    KeyCode::Esc => {
                        self.overlay = None;
                        self.help_filtering = false;
                        self.help_filter.clear();
                    }
                    KeyCode::Enter => self.help_filtering = false,
                    KeyCode::Backspace => {
                        self.help_filter.pop();
                    }
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.help_filter.push(c);
                    }
                    _ => {}
                }
                return false;
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('n')
                    if matches!(overlay, Overlay::ConfirmUninstall | Overlay::ConfirmUpdate) =>
                {
                    self.overlay = None;
                    self.uninstall_targets.clear();
                    self.uninstall_commands.clear();
                }
                KeyCode::Enter | KeyCode::Char('y')
                    if matches!(overlay, Overlay::ConfirmUninstall | Overlay::ConfirmUpdate) =>
                {
                    self.overlay = None;
                    self.start_operation();
                }
                KeyCode::Esc if overlay == Overlay::Help => {
                    self.overlay = None;
                    self.help_filtering = false;
                    self.help_filter.clear();
                }
                KeyCode::Char('q') | KeyCode::Enter if overlay == Overlay::Help => {
                    self.overlay = None
                }
                KeyCode::Char('?') if overlay == Overlay::Help => self.overlay = None,
                KeyCode::Char('/') if overlay == Overlay::Help => {
                    self.help_filter.clear();
                    self.help_filtering = true;
                }
                _ => {}
            }
            return false;
        }

        if key.code == KeyCode::Esc
            && self
                .uninstall_console
                .as_ref()
                .is_some_and(|console| !console.running)
        {
            if self.focus == Focus::Details {
                self.focus = Focus::Packages;
                self.refresh_after_uninstall_console();
            } else {
                self.uninstall_console = None;
                self.detail_scroll = 0;
            }
            return false;
        }

        let keymap = &self.config.keymap;
        let action = keymap.resolve_action(&key);
        match action {
            Some(Action::Quit) => return true,
            Some(Action::Help) => self.overlay = Some(Overlay::Help),
            Some(Action::Search) => {
                self.searching = true;
                self.focus = Focus::Packages;
            }
            Some(Action::ClearSearch) => {
                if !self.query.is_empty() {
                    self.query.clear();
                    self.package_index = 0;
                    self.update_filter();
                }
            }
            Some(Action::SortToggle) => {
                self.sort = self.sort.next();
                self.sort.sort_packages(&mut self.packages);
                self.update_filter();
            }
            Some(Action::ToggleUpdates) => self.toggle_updates_view(),
            Some(Action::TogglePackageSelection) => self.toggle_package_selection(),
            Some(Action::ToggleDependencies) => {
                self.show_dependencies = !self.show_dependencies;
                self.package_index = 0;
                self.update_filter();
            }
            Some(Action::TogglePackageDependencies) => {
                if let Some(package) = self.selected_package() {
                    if package.dependencies.is_empty() {
                        self.set_notice("Selected package has no dependency data");
                    } else {
                        self.show_package_dependencies = !self.show_package_dependencies;
                        self.dependency_index = 0;
                        self.focus = if self.show_package_dependencies {
                            Focus::PackageDependencies
                        } else {
                            Focus::Packages
                        };
                    }
                }
            }
            Some(Action::Reload) => {
                if self.focus == Focus::Details
                    && self
                        .uninstall_console
                        .as_ref()
                        .is_some_and(|console| !console.running)
                {
                    self.uninstall_console = None;
                    self.detail_scroll = 0;
                    self.focus = Focus::Packages;
                }
                if self.updates_only {
                    self.updates_checked = false;
                    self.update_versions.clear();
                }
                self.start_reload();
            }
            Some(Action::Uninstall) => self.prepare_uninstall(),
            Some(Action::UpdatePackages) => self.prepare_update(),
            Some(Action::AddProjectPackage) => {
                if self.config.manager.projects.enabled && self.active_project().is_some() {
                    self.project_add_input = Some(String::new());
                } else {
                    self.set_notice("Select a project manager to add a project dependency");
                }
            }
            Some(Action::ClearPackageSelection) => {
                let cleared = self.selected_packages.len();
                self.selected_packages.clear();
                self.set_notice(format!("Cleared selection ({cleared})"));
            }
            Some(Action::GotoAll) => self.select_tab(FilterTab::All),
            Some(Action::GotoPacman) => self.select_tab(FilterTab::Pacman),
            Some(Action::GotoAur) => self.select_tab(FilterTab::Aur),
            Some(Action::GotoFlatpak) => self.select_tab(FilterTab::Flatpak),
            Some(Action::GotoCargo) => self.select_tab(FilterTab::Cargo),
            Some(Action::GotoPython) => self.select_tab(FilterTab::Python),
            Some(Action::FocusLeft) => self.move_focus(-1),
            Some(Action::FocusRight) => self.move_focus(1),
            Some(Action::FocusNext) => self.move_focus(1),
            Some(Action::FocusPrev) => self.move_focus(-1),
            Some(Action::Down) => self.move_selection(1),
            Some(Action::Up) => self.move_selection(-1),
            Some(Action::PageDown) => self.move_selection(10),
            Some(Action::PageUp) => self.move_selection(-10),
            Some(Action::Top) => self.go_to_edge(false),
            Some(Action::Bottom) => self.go_to_edge(true),
            Some(Action::ToggleManagers) => {
                self.config.layout.show_managers = !self.config.layout.show_managers;
                if !self.config.layout.show_managers && self.focus == Focus::Managers {
                    self.focus = Focus::Packages;
                }
            }
            Some(Action::ToggleDetails) => {
                self.config.layout.show_details = !self.config.layout.show_details;
                if !self.config.layout.show_details && self.focus == Focus::Details {
                    self.focus = Focus::Packages;
                }
            }
            None => {
                // Unbound keys have no effect.
            }
        }
        false
    }

    fn select_tab(&mut self, tab: FilterTab) {
        self.tab = tab;
        self.manager_index = tab.index();
        self.active_project_manager = None;
        self.package_index = 0;
        self.focus = Focus::Packages;
        self.update_filter();
    }

    fn toggle_package_selection(&mut self) {
        if self.focus != Focus::Packages {
            return;
        }
        if let Some(package) = self.selected_package().cloned() {
            let identity = package_identity(&package);
            if !self.selected_packages.insert(identity.clone()) {
                self.selected_packages.remove(&identity);
            }
            if !self.filtered.is_empty() {
                self.package_index = (self.package_index + 1).min(self.filtered.len() - 1);
            }
        }
    }

    fn toggle_updates_view(&mut self) {
        self.updates_only = !self.updates_only;
        self.package_index = 0;
        self.focus = Focus::Packages;
        if !self.loading && !self.updates_checked && !self.checking_updates {
            self.start_update_check();
        }
        self.update_filter();
    }

    fn start_update_check(&mut self) {
        if self.checking_updates || self.loading {
            return;
        }
        self.checking_updates = true;
        let tx = self.reload_tx.clone();
        let packages = self.packages.clone();
        let generation = self.inventory_generation;
        thread::spawn(move || {
            let _ = tx.send(check_available_updates(generation, packages));
        });
    }

    fn maybe_check_updates_after_scan(&mut self) {
        if !self.updates_checked && !self.checking_updates {
            self.start_update_check();
        }
    }

    fn move_focus(&mut self, direction: i8) {
        let panes = [
            self.config.layout.show_managers.then_some(Focus::Managers),
            Some(Focus::Packages),
            (self.show_package_dependencies
                && self
                    .selected_package()
                    .is_some_and(|p| !p.dependencies.is_empty()))
            .then_some(Focus::PackageDependencies),
            self.config.layout.show_details.then_some(Focus::Details),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        let current = panes
            .iter()
            .position(|pane| *pane == self.focus)
            .unwrap_or(0);
        let next = (current as isize + direction as isize).clamp(0, panes.len() as isize - 1);
        let previous = self.focus;
        self.focus = panes[next as usize];
        if previous == Focus::Details
            && self.focus != Focus::Details
            && self
                .uninstall_console
                .as_ref()
                .is_some_and(|console| !console.running)
        {
            self.refresh_after_uninstall_console();
        }
    }

    fn refresh_after_uninstall_console(&mut self) {
        if self
            .uninstall_console
            .as_ref()
            .is_some_and(|console| !console.running)
        {
            self.uninstall_console = None;
            self.detail_scroll = 0;
            self.start_reload();
        }
    }

    fn move_selection(&mut self, amount: isize) {
        match self.focus {
            Focus::Managers => {
                let last = FilterTab::ALL.len() + self.project_managers.len() - 1;
                let next = (self.manager_index as isize + amount).clamp(0, last as isize);
                self.select_manager_index(next as usize);
            }
            Focus::Packages => {
                if !self.filtered.is_empty() {
                    self.package_index = (self.package_index as isize + amount)
                        .clamp(0, self.filtered.len() as isize - 1)
                        as usize;
                    self.dependency_index = 0;
                }
            }
            Focus::PackageDependencies => {
                let len = self
                    .selected_package()
                    .map_or(0, |package| package.dependencies.len());
                if len > 0 {
                    self.dependency_index = (self.dependency_index as isize + amount)
                        .clamp(0, len as isize - 1)
                        as usize;
                    self.detail_scroll = 0;
                }
            }
            Focus::Details => {
                if self.uninstall_console.is_some() {
                    self.console_scroll = self.console_scroll.saturating_add_signed(amount);
                } else {
                    self.detail_scroll = self.detail_scroll.saturating_add_signed(amount as i16)
                }
            }
        }
    }

    fn go_to_edge(&mut self, bottom: bool) {
        match self.focus {
            Focus::Managers => self.select_manager_index(if bottom {
                FilterTab::ALL.len() + self.project_managers.len() - 1
            } else {
                0
            }),
            Focus::Packages => {
                self.package_index = if bottom {
                    self.filtered.len().saturating_sub(1)
                } else {
                    0
                }
            }
            Focus::PackageDependencies => {
                let len = self
                    .selected_package()
                    .map_or(0, |package| package.dependencies.len());
                self.dependency_index = if bottom { len.saturating_sub(1) } else { 0 };
            }
            Focus::Details => {
                if self.uninstall_console.is_some() {
                    self.console_scroll = if bottom { usize::MAX } else { 0 };
                } else {
                    self.detail_scroll = if bottom { u16::MAX } else { 0 };
                }
            }
        }
    }

    fn start_reload(&mut self) {
        if self.loading {
            return;
        }
        self.loading = true;
        self.inventory_generation = self.inventory_generation.wrapping_add(1);
        self.updates_checked = false;
        self.checking_updates = false;
        self.update_versions.clear();
        for package in &mut self.packages {
            package.latest_version = None;
        }
        // The provider scan is intentionally detached from the event loop; the
        // UI remains responsive and receives its result through the channel.
        let tx = self.reload_tx.clone();
        let custom = self.config.custom_managers.clone();
        let projects = self.config.manager.projects.clone();
        thread::spawn(move || {
            let (packages, errors, contexts) = scan_packages(custom, projects);
            let _ = tx.send(UiEvent::Packages((packages, errors, contexts)));
        });
    }

    fn prepare_uninstall(&mut self) {
        if self
            .uninstall_console
            .as_ref()
            .is_some_and(|console| console.running)
        {
            self.set_notice("Wait for the current package operation to finish");
            return;
        }
        let targets = if self.focus == Focus::Packages && !self.selected_packages.is_empty() {
            self.packages
                .iter()
                .filter(|package| self.selected_packages.contains(&package_identity(package)))
                .cloned()
                .collect::<Vec<_>>()
        } else if matches!(self.focus, Focus::PackageDependencies | Focus::Details)
            && self.show_package_dependencies
        {
            self.selected_dependency_package()
                .cloned()
                .into_iter()
                .collect()
        } else {
            self.selected_package().cloned().into_iter().collect()
        };
        if targets.is_empty() {
            self.set_notice("Dependency is not present in the installed package inventory");
            return;
        }
        let commands = targets
            .iter()
            .map(|target| {
                build_uninstall_command_with_projects(
                    target,
                    &self.config.behavior,
                    &self.config.custom_managers,
                    &self.config.manager.projects,
                )
            })
            .collect::<Result<Vec<_>, _>>();
        match commands {
            Ok(commands) => {
                self.uninstall_targets = targets;
                self.uninstall_commands = commands;
                self.operation_kind = OperationKind::Uninstall;
                self.selected_packages.clear();
                if self.config.behavior.confirm_uninstall {
                    self.overlay = Some(Overlay::ConfirmUninstall);
                } else {
                    self.start_operation();
                }
            }
            Err(error) => self.set_notice(error),
        }
    }

    fn start_project_add(&mut self, name: String) {
        if self
            .uninstall_console
            .as_ref()
            .is_some_and(|console| console.running)
        {
            self.set_notice("Wait for the current package operation to finish");
            return;
        }
        let Some(project) = self.active_project().cloned() else {
            self.set_notice("Select a project manager before adding a dependency");
            return;
        };
        if name.trim().is_empty() {
            self.set_notice("Package name cannot be empty");
            return;
        }
        match build_project_add_command(
            &project.manager_id,
            &project.root,
            name.trim(),
            &self.config.manager.projects,
        ) {
            Ok(command) => {
                let mut package = UnifiedPackage::new(
                    name.trim(),
                    "new dependency",
                    crate::model::PackageSource::Project,
                );
                package.manager_id = Some(project.manager_id);
                self.uninstall_targets = vec![package];
                self.uninstall_commands = vec![command];
                self.operation_kind = OperationKind::Add;
                self.launch_operation(None);
            }
            Err(error) => self.set_notice(error),
        }
    }

    fn prepare_update(&mut self) {
        if self
            .uninstall_console
            .as_ref()
            .is_some_and(|console| console.running)
        {
            self.set_notice("Wait for the current package operation to finish");
            return;
        }
        if self.loading {
            self.set_notice("Package inventory is still loading");
            return;
        }
        if self.checking_updates || !self.updates_checked {
            self.set_notice("Update check is still running");
            return;
        }
        let targets = if !self.selected_packages.is_empty() {
            self.packages
                .iter()
                .filter(|package| self.selected_packages.contains(&package_identity(package)))
                .cloned()
                .collect::<Vec<_>>()
        } else {
            self.selected_package().cloned().into_iter().collect()
        };
        if targets.is_empty() {
            self.set_notice("No package is selected");
            return;
        }
        let mut updatable = targets
            .into_iter()
            .filter(|package| {
                self.update_versions
                    .contains_key(&package_identity(package))
            })
            .collect::<Vec<_>>();
        if updatable.is_empty() {
            self.set_notice("No available update for the selected package(s)");
            return;
        }
        let skipped = if self.selected_packages.is_empty() {
            0
        } else {
            self.selected_packages.len().saturating_sub(updatable.len())
        };
        let mut commands = Vec::new();
        for source in [
            crate::model::PackageSource::Pacman,
            crate::model::PackageSource::Aur,
        ] {
            let group = updatable
                .iter()
                .filter(|package| package.source == source)
                .cloned()
                .collect::<Vec<_>>();
            if !group.is_empty() {
                match build_update_command(&group, &self.config.behavior) {
                    Ok(command) => commands.push(command),
                    Err(error) => {
                        self.set_notice(error);
                        return;
                    }
                }
            }
        }
        if commands.is_empty() {
            self.set_notice("Updates are currently supported for Pacman and AUR only");
            return;
        }
        if skipped > 0 {
            self.set_notice(format!(
                "Updating {} package(s); {skipped} selected package(s) have no supported update",
                updatable.len()
            ));
        }
        self.uninstall_targets = std::mem::take(&mut updatable);
        self.uninstall_commands = commands;
        self.operation_kind = OperationKind::Update;
        self.selected_packages.clear();
        self.overlay = Some(Overlay::ConfirmUpdate);
    }

    fn start_operation(&mut self) {
        if self
            .uninstall_commands
            .iter()
            .any(|command| command.requires_stdin_password())
        {
            self.sudo_password_input = Some(Zeroizing::new(Vec::new()));
            return;
        }
        self.launch_operation(None);
    }

    fn launch_operation(&mut self, password: Option<Zeroizing<Vec<u8>>>) {
        if self.uninstall_targets.is_empty() || self.uninstall_commands.is_empty() {
            return;
        }
        let operation = self.operation_kind;
        let targets = std::mem::take(&mut self.uninstall_targets);
        let commands = std::mem::take(&mut self.uninstall_commands);
        let tx = self.reload_tx.clone();
        let package_names = targets
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let displayed_commands = commands
            .iter()
            .map(|command| format!("$ {}", command.display))
            .collect::<Vec<_>>();
        self.uninstall_console = Some(UninstallConsole {
            operation,
            package: package_names,
            lines: displayed_commands
                .into_iter()
                .chain(std::iter::once(String::new()))
                .collect(),
            running: true,
            exit_code: None,
        });
        self.console_scroll = usize::MAX;
        self.focus = Focus::Details;
        thread::spawn(move || {
            let mut final_code = Some(0);
            for command in commands {
                let uses_stdin_password = password.is_some() && command.requires_stdin_password();
                let mut process = if uses_stdin_password {
                    command.command_with_stdin_password()
                } else {
                    command.command()
                };
                process
                    .stdin(if uses_stdin_password {
                        std::process::Stdio::piped()
                    } else {
                        std::process::Stdio::null()
                    })
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped());
                let mut child = match process.spawn() {
                    Ok(child) => child,
                    Err(error) => {
                        let _ = tx.send(UiEvent::OperationOutput(format!(
                            "Failed to start command: {error}"
                        )));
                        final_code = None;
                        break;
                    }
                };
                if uses_stdin_password {
                    if let (Some(secret), Some(mut stdin)) = (password.as_ref(), child.stdin.take())
                    {
                        let write_result = stdin
                            .write_all(secret.as_slice())
                            .and_then(|()| stdin.write_all(b"\n"));
                        drop(stdin);
                        if let Err(error) = write_result {
                            let _ = child.kill();
                            let _ = child.wait();
                            let _ = tx.send(UiEvent::OperationOutput(format!(
                                "Failed to send password to sudo: {error}"
                            )));
                            final_code = None;
                            break;
                        }
                    }
                }
                let stdout = child.stdout.take();
                let stderr = child.stderr.take();
                let out_tx = tx.clone();
                let err_tx = tx.clone();
                let out_reader = thread::spawn(move || read_output(stdout, out_tx));
                let err_reader = thread::spawn(move || read_output(stderr, err_tx));
                let status = child.wait().ok();
                let _ = out_reader.join();
                let _ = err_reader.join();
                let code = status.and_then(|status| status.code());
                if code != Some(0) {
                    final_code = code;
                }
            }
            if let Some(mut password) = password {
                password.zeroize();
            }
            let _ = tx.send(UiEvent::OperationFinished(final_code));
        });
    }

    fn apply_config(&mut self, config: OmniConfig, managers_changed: bool) {
        let project_managers_changed = self.config.manager.projects != config.manager.projects;
        self.config = config;
        if !self.config.layout.show_managers && self.focus == Focus::Managers {
            self.focus = Focus::Packages;
        }
        if !self.config.layout.show_details && self.focus == Focus::Details {
            self.focus = Focus::Packages;
        }
        self.set_notice("Configuration reloaded");
        if managers_changed || project_managers_changed {
            if self.loading {
                self.reload_after_loading = true;
            } else {
                self.start_reload();
            }
        }
    }

    fn set_notice(&mut self, message: impl Into<String>) {
        self.notice = Some((message.into(), Instant::now() + Duration::from_secs(3)));
    }

    fn show_next_notice(&mut self) {
        let Some(message) = (!self.notice_queue.is_empty()).then(|| self.notice_queue.remove(0))
        else {
            return;
        };
        self.notice = Some((message, Instant::now() + Duration::from_secs(4)));
    }

    fn apply_ui_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::Packages((mut packages, errors, project_managers)) => {
                self.sort.sort_packages(&mut packages);
                self.packages = packages;
                self.project_managers = project_managers;
                if self.active_project_manager.as_ref().is_some_and(|active| {
                    !self
                        .project_managers
                        .iter()
                        .any(|project| &project.manager_id == active)
                }) {
                    self.active_project_manager = None;
                    self.manager_index = 0;
                }
                self.loading = false;
                self.update_filter();
                if !errors.is_empty() {
                    self.notice_queue = errors
                        .into_iter()
                        .map(|error| format!("Package scan error — {error}"))
                        .collect();
                    self.show_next_notice();
                }
                if self.reload_after_loading {
                    self.reload_after_loading = false;
                    self.start_reload();
                } else {
                    self.maybe_check_updates_after_scan();
                }
            }
            UiEvent::OperationOutput(line) => {
                if let Some(console) = self.uninstall_console.as_mut() {
                    console.lines.push(line);
                    if console.lines.len() > 2_000 {
                        let excess = console.lines.len() - 2_000;
                        console.lines.drain(..excess);
                    }
                    self.console_scroll = usize::MAX;
                }
            }
            UiEvent::OperationFinished(code) => {
                let operation = self
                    .uninstall_console
                    .as_ref()
                    .map(|console| console.operation)
                    .unwrap_or(self.operation_kind);
                if let Some(console) = self.uninstall_console.as_mut() {
                    console.running = false;
                    console.exit_code = code;
                    console.lines.push(match code {
                        Some(0) => "\nCommand completed successfully.".into(),
                        Some(code) => format!("\nCommand exited with status {code}."),
                        None => "\nCommand terminated without an exit status.".into(),
                    });
                }
                if code == Some(0) {
                    self.set_notice(format!(
                        "{} completed; press r or leave Details to refresh packages",
                        operation.label()
                    ));
                } else {
                    self.set_notice(format!(
                        "{} failed; see output in Details",
                        operation.label()
                    ));
                }
            }
            UiEvent::Updates(generation, updates, errors) => {
                if generation != self.inventory_generation {
                    return;
                }
                self.update_versions = updates;
                self.updates_checked = true;
                self.checking_updates = false;
                for package in &mut self.packages {
                    package.latest_version = self
                        .update_versions
                        .get(&package_identity(package))
                        .cloned();
                }
                self.update_filter();
                if !errors.is_empty() {
                    self.notice_queue = errors;
                    self.show_next_notice();
                }
            }
        }
    }
}

fn read_output<R: std::io::Read + Send + 'static>(reader: Option<R>, tx: Sender<UiEvent>) {
    let Some(reader) = reader else {
        return;
    };
    for line in BufReader::new(reader).lines() {
        match line {
            Ok(line) => {
                let _ = tx.send(UiEvent::OperationOutput(line));
            }
            Err(error) => {
                let _ = tx.send(UiEvent::OperationOutput(format!(
                    "Output read error: {error}"
                )));
                break;
            }
        }
    }
}

fn dependency_package_name(value: &str) -> &str {
    let end = value
        .find(|character: char| matches!(character, '<' | '>' | '='))
        .unwrap_or(value.len());
    value[..end].trim_end()
}

pub fn run(config: OmniConfig) -> Result<()> {
    run_with_config_path(config, None)
}

pub fn run_with_config_path(config: OmniConfig, explicit_config: Option<PathBuf>) -> Result<()> {
    let mut terminal = TerminalGuard::enter()?;
    let (tx, rx) = mpsc::channel();
    let app_tx = tx.clone();
    let custom = config.custom_managers.clone();
    let projects = config.manager.projects.clone();
    thread::spawn(move || {
        let _ = tx.send(UiEvent::Packages(scan_packages(custom, projects)));
    });
    let mut app = App::new(config, app_tx);
    let config_files = ConfigFiles::new(explicit_config);
    let mut watcher = ConfigWatcher::new(config_files);
    let result = event_loop(&mut terminal.terminal, &mut app, &rx, &mut watcher);
    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    rx: &Receiver<UiEvent>,
    watcher: &mut ConfigWatcher,
) -> Result<()> {
    loop {
        for event in rx.try_iter() {
            app.apply_ui_event(event);
        }
        if app
            .notice
            .as_ref()
            .is_some_and(|(_, expires)| *expires <= Instant::now())
        {
            app.notice = None;
            app.show_next_notice();
        }
        if let Some(result) = watcher.poll() {
            match result {
                Ok((config, managers_changed)) => app.apply_config(config, managers_changed),
                Err(error) => app.set_notice(format!("Config error: {error}")),
            }
        }
        terminal.draw(|frame| render(frame, app))?;
        if event::poll(Duration::from_millis(if app.loading { 80 } else { 250 }))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Release {
                    continue;
                }
                if app.handle_key(key) {
                    break;
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileFingerprint {
    modified: Option<SystemTime>,
    length: u64,
}

struct ConfigFiles {
    config: Option<PathBuf>,
    theme: Option<PathBuf>,
    keymap: Option<PathBuf>,
    managers: Option<PathBuf>,
    managers_overlay: bool,
}

impl ConfigFiles {
    fn new(explicit_config: Option<PathBuf>) -> Self {
        if let Some(config) = explicit_config {
            let managers = config.parent().map(|dir| dir.join("managers.toml"));
            Self {
                config: Some(config),
                theme: None,
                keymap: None,
                managers,
                managers_overlay: true,
            }
        } else {
            Self {
                config: default_config_path(),
                theme: default_theme_path(),
                keymap: default_keymap_path(),
                managers: default_managers_path(),
                managers_overlay: false,
            }
        }
    }

    fn paths(&self) -> Vec<PathBuf> {
        [
            self.config.as_ref(),
            self.theme.as_ref(),
            self.keymap.as_ref(),
            self.managers.as_ref(),
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect()
    }

    fn load(&self) -> Result<OmniConfig, String> {
        let mut config = match self.config.as_deref() {
            Some(path) if path.exists() => {
                let contents = read_config_file(path)?;
                toml::from_str(&contents).map_err(|error| format!("{}: {error}", path.display()))?
            }
            _ => OmniConfig::default(),
        };

        if let Some(path) = self.theme.as_deref()
            && path.exists()
        {
            config.theme = parse_theme_toml(&read_config_file(path)?)
                .map_err(|error| format!("{}: {error}", path.display()))?;
        }
        if let Some(path) = self.keymap.as_deref()
            && path.exists()
        {
            config.keymap = parse_keymap_toml(&read_config_file(path)?)
                .map_err(|error| format!("{}: {error}", path.display()))?;
        }
        if let Some(path) = self.managers.as_deref()
            && path.exists()
        {
            let managers = parse_managers_toml(&read_config_file(path)?)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            if self.managers_overlay {
                config.custom_managers.extend(managers);
            } else {
                config.custom_managers = managers;
            }
        }
        Ok(config)
    }
}

struct ConfigWatcher {
    files: ConfigFiles,
    fingerprints: Vec<Option<FileFingerprint>>,
    last_check: Instant,
    pending_managers_reload: bool,
}

impl ConfigWatcher {
    fn new(files: ConfigFiles) -> Self {
        let fingerprints = files
            .paths()
            .iter()
            .map(|path| fingerprint(path).ok().flatten())
            .collect();
        Self {
            files,
            fingerprints,
            last_check: Instant::now(),
            pending_managers_reload: false,
        }
    }

    fn poll(&mut self) -> Option<Result<(OmniConfig, bool), String>> {
        if self.last_check.elapsed() < Duration::from_millis(350) {
            return None;
        }
        self.last_check = Instant::now();
        let paths = self.files.paths();
        let mut changed = false;
        let mut managers_changed = false;
        for (index, path) in paths.iter().enumerate() {
            let current = match fingerprint(path) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if current != self.fingerprints[index] {
                changed = true;
                if self.files.managers.as_ref() == Some(path) {
                    self.pending_managers_reload = true;
                }
                self.fingerprints[index] = current;
            }
        }
        if !changed {
            return None;
        }
        let result = self.files.load().map(|config| {
            managers_changed = self.pending_managers_reload;
            self.pending_managers_reload = false;
            (config, managers_changed)
        });
        Some(result)
    }
}

fn fingerprint(path: &Path) -> std::io::Result<Option<FileFingerprint>> {
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(Some(FileFingerprint {
            modified: metadata.modified().ok(),
            length: metadata.len(),
        })),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn read_config_file(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))
}

fn scan_packages(
    custom_configs: Vec<crate::config::CustomManagerConfig>,
    project_config: ProjectManagersConfig,
) -> (Vec<UnifiedPackage>, Vec<String>, Vec<ProjectContext>) {
    let providers: Vec<Box<dyn PackageManagerProvider>> = vec![
        Box::new(PacmanProvider),
        Box::new(YayProvider),
        Box::new(FlatpakProvider),
        Box::new(CargoProvider),
        Box::new(UvProvider),
        Box::new(PipProvider),
    ];
    let mut packages = Vec::new();
    let mut errors = Vec::new();
    for provider in providers {
        if provider.is_available() {
            match provider.list_packages() {
                Ok(mut found) => packages.append(&mut found),
                Err(error) => errors.push(format!("{}: {error}", provider.id())),
            }
        }
    }
    for provider in CustomManager::from_configs(custom_configs) {
        if provider.is_available() {
            match provider.list_packages() {
                Ok(mut found) => packages.append(&mut found),
                Err(error) => errors.push(format!("{}: {error}", provider.id())),
            }
        }
    }
    let project_managers = if project_config.enabled {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let current = cwd.canonicalize().unwrap_or(cwd);
        let mut found = discover_projects(&current);
        if !project_config.detect_ancestors {
            found.retain(|project| project.root == current);
        }
        found.retain(|project| project_config.supports(project.ecosystem));
        for project in &found {
            packages.extend(project.packages.clone());
        }
        found
    } else {
        Vec::new()
    };
    (packages, errors, project_managers)
}

fn check_available_updates(generation: u64, packages: Vec<UnifiedPackage>) -> UiEvent {
    let mut updates = HashMap::new();
    let mut errors = Vec::new();
    for (binary, args, source) in [
        (
            "checkupdates",
            Vec::<&str>::new(),
            crate::model::PackageSource::Pacman,
        ),
        ("yay", vec!["-Qua"], crate::model::PackageSource::Aur),
    ] {
        if !packages.iter().any(|package| package.source == source) {
            continue;
        }
        let output = match std::process::Command::new(binary).args(args).output() {
            Ok(output) => output,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if packages.iter().any(|package| package.source == source) {
                    errors.push(match binary {
                        "checkupdates" => {
                            "Install pacman-contrib to check repository updates".to_owned()
                        }
                        "yay" => "yay is unavailable; AUR updates could not be checked".to_owned(),
                        _ => format!("{binary} is unavailable"),
                    });
                }
                continue;
            }
            Err(error) => {
                errors.push(format!("Could not start update check ({binary}): {error}"));
                continue;
            }
        };
        if !output.status.success()
            && !(binary == "checkupdates" && output.status.code() == Some(2))
        {
            errors.push(format!(
                "Update check ({binary}) failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
            continue;
        }
        let installed_names = packages
            .iter()
            .filter(|package| package.source == source)
            .map(|package| package.name.as_str())
            .collect::<HashSet<_>>();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let mut fields = line.split_whitespace();
            let Some(name) = fields.next() else { continue };
            let Some(_installed_version) = fields.next() else {
                continue;
            };
            if fields.next() != Some("->") {
                continue;
            }
            let Some(latest_version) = fields.next() else {
                continue;
            };
            if installed_names.contains(name) {
                let key = (source, None, name.to_owned());
                updates.insert(key, latest_version.to_owned());
            }
        }
    }
    UiEvent::Updates(generation, updates, errors)
}

fn render(frame: &mut Frame, app: &App) {
    let theme = &app.config.theme.colors;
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.bg_color())),
        frame.area(),
    );
    let areas = layout::split(frame.area(), &app.config.layout);
    if let Some(area) = areas.managers {
        render_managers(frame, area, app, theme);
    }
    render_packages(frame, areas.packages, app, theme);
    if let Some(area) = areas.details {
        render_details(frame, area, app, theme);
    }
    render_dividers(frame, &areas.dividers, theme);
    let total_size = app
        .filtered
        .iter()
        .filter_map(|index| app.packages[*index].size_bytes)
        .sum();
    statusline::render(
        frame,
        areas.status,
        theme,
        app.searching,
        &app.query,
        app.active_project()
            .map(|project| project.label.as_str())
            .unwrap_or_else(|| app.tab.display_name()),
        app.package_index,
        app.filtered.len(),
        total_size,
        app.sort,
        app.show_dependencies,
        app.notice
            .as_ref()
            .filter(|(_, expires)| *expires > Instant::now())
            .map(|(message, _)| message.as_str()),
        app.loading || app.checking_updates,
        (app.started_at.elapsed().as_millis() as usize / 80) % 10,
        app.update_versions.len(),
        app.updates_only,
        app.selected_packages.len(),
    );
    if let Some(password) = app.sudo_password_input.as_ref() {
        render_sudo_password_prompt(frame, areas.status, password.len(), theme);
    }
    if let Some(input) = app.project_add_input.as_ref() {
        render_project_add_prompt(
            frame,
            areas.status,
            input,
            app.active_project()
                .map(|project| project.label.as_str())
                .unwrap_or("project"),
            theme,
        );
    }
    match app.overlay {
        Some(Overlay::Help) => {
            let dependency_key = app
                .config
                .keymap
                .get_bindings(Action::ToggleDependencies)
                .format_summary();
            let package_dependency_key = app
                .config
                .keymap
                .get_bindings(Action::TogglePackageDependencies)
                .format_summary();
            let updates_key = app
                .config
                .keymap
                .get_bindings(Action::ToggleUpdates)
                .format_summary();
            let selection_key = app
                .config
                .keymap
                .get_bindings(Action::TogglePackageSelection)
                .format_summary();
            let clear_selection_key = app
                .config
                .keymap
                .get_bindings(Action::ClearPackageSelection)
                .format_summary();
            let update_packages_key = app
                .config
                .keymap
                .get_bindings(Action::UpdatePackages)
                .format_summary();
            let add_project_key = app
                .config
                .keymap
                .get_bindings(Action::AddProjectPackage)
                .format_summary();
            help::render(
                frame,
                frame.area(),
                theme,
                app.config.layout.popup_border_type(),
                &dependency_key,
                &package_dependency_key,
                &updates_key,
                &selection_key,
                &clear_selection_key,
                &update_packages_key,
                &add_project_key,
                &app.help_filter,
                app.help_filtering,
            )
        }
        Some(Overlay::ConfirmUninstall | Overlay::ConfirmUpdate) => {
            if !app.uninstall_targets.is_empty() && !app.uninstall_commands.is_empty() {
                let commands = app
                    .uninstall_commands
                    .iter()
                    .map(|command| command.display.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                render_operation_confirm(
                    frame,
                    frame.area(),
                    app.operation_kind,
                    &app.uninstall_targets,
                    &commands,
                    theme,
                    app.config.layout.popup_border_type(),
                );
            }
        }
        None => {}
    }
}

fn render_managers(frame: &mut Frame, area: Rect, app: &App, theme: &ThemeColors) {
    let panel_bg = theme.panel_managers_bg_color();
    let content_area = draw_panel(
        frame,
        area,
        if app.focus == Focus::Managers {
            " MANAGERS "
        } else {
            " Managers "
        },
        app.focus == Focus::Managers,
        app,
        theme,
        panel_bg,
    );
    let counts = app.manager_counts();
    let update_counts: [usize; 6] = FilterTab::ALL.map(|tab| {
        app.packages
            .iter()
            .filter(|package| tab.matches(package.source))
            .filter(|package| package.latest_version.is_some())
            .filter(|package| {
                app.show_dependencies || package.install_reason != InstallReason::Dependency
            })
            .filter(|package| {
                app.query.is_empty()
                    || SkimMatcherV2::default()
                        .fuzzy_match(&package.name, &app.query)
                        .is_some()
            })
            .count()
    });
    let selected_style = theme.selection_style().add_modifier(Modifier::BOLD);
    let mut items = FilterTab::ALL
        .iter()
        .enumerate()
        .map(|(index, tab)| {
            let selected = index == app.manager_index;
            let mut spans = vec![
                Span::styled(
                    if selected { "▍" } else { " " },
                    if selected {
                        Style::default().fg(theme.cursor_bar_color())
                    } else {
                        Style::default()
                    },
                ),
                Span::raw(format!(" {:<9} {:>5}", tab.display_name(), counts[index])),
            ];
            if update_counts[index] > 0 {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    format!("↑{}", update_counts[index]),
                    Style::default()
                        .fg(theme.border_focused_color())
                        .add_modifier(Modifier::BOLD),
                ));
            }
            let line = Line::from(spans);
            ListItem::new(line).style(if selected {
                selected_style
            } else {
                Style::default().fg(theme.fg_color())
            })
        })
        .collect::<Vec<_>>();
    for (project_index, project) in app.project_managers.iter().enumerate() {
        let index = FilterTab::ALL.len() + project_index;
        let selected = index == app.manager_index;
        let count = project
            .packages
            .iter()
            .filter(|package| {
                app.show_dependencies || package.install_reason != InstallReason::Dependency
            })
            .filter(|package| {
                app.query.is_empty()
                    || SkimMatcherV2::default()
                        .fuzzy_match(&package.name, &app.query)
                        .is_some()
            })
            .count();
        let update_count = project
            .packages
            .iter()
            .filter(|package| package.latest_version.is_some())
            .filter(|package| {
                app.show_dependencies || package.install_reason != InstallReason::Dependency
            })
            .filter(|package| {
                app.query.is_empty()
                    || SkimMatcherV2::default()
                        .fuzzy_match(&package.name, &app.query)
                        .is_some()
            })
            .count();
        let mut spans = vec![
            Span::styled(
                if selected { "▍" } else { " " },
                if selected {
                    Style::default().fg(theme.cursor_bar_color())
                } else {
                    Style::default()
                },
            ),
            Span::raw(format!(" {:<15} {:>5}", project.label, count)),
        ];
        if update_count > 0 {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!("↑{update_count}"),
                Style::default()
                    .fg(theme.border_focused_color())
                    .add_modifier(Modifier::BOLD),
            ));
        }
        let line = Line::from(spans);
        items.push(ListItem::new(line).style(if selected {
            selected_style
        } else {
            Style::default().fg(theme.fg_color())
        }));
    }
    let mut state = ListState::default().with_selected(Some(app.manager_index));
    frame.render_stateful_widget(
        List::new(items).style(Style::default().fg(theme.fg_color()).bg(panel_bg)),
        content_area,
        &mut state,
    );
}

fn render_packages(frame: &mut Frame, area: Rect, app: &App, theme: &ThemeColors) {
    let panel_bg = theme.panel_packages_bg_color();
    let title = if app.searching {
        format!(" PACKAGES /{} ", app.query)
    } else if app.updates_only {
        format!(" UPDATES ({}) ", app.update_versions.len())
    } else if let Some(project) = app.active_project() {
        format!(" PROJECT ({}) ", project.label)
    } else {
        format!(" PACKAGES ({}) ", app.tab.display_name())
    };
    let content_area = draw_panel(
        frame,
        area,
        &title,
        app.focus == Focus::Packages,
        app,
        theme,
        panel_bg,
    );
    let (list_area, deps_area) = if app.show_package_dependencies {
        let rows = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Min(1),
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Length(7),
            ])
            .split(content_area);
        (rows[0], Some((rows[1], rows[2])))
    } else {
        (content_area, None)
    };
    let items = app
        .filtered
        .iter()
        .enumerate()
        .map(|(position, index)| {
            let package = &app.packages[*index];
            let is_selected = position == app.package_index;
            let marker = if is_selected { "▍" } else { " " };
            let checked = if app.selected_packages.contains(&package_identity(package)) {
                "✓"
            } else {
                " "
            };
            let has_update = package.latest_version.is_some();
            let size = package
                .size_bytes
                .map(|bytes| format_bytes(Some(bytes)))
                .unwrap_or_else(|| "—".into());
            let available_width = content_area.width.saturating_sub(18) as usize;
            let name = truncate(&package.name, available_width);
            let spans = vec![
                Span::styled(marker, Style::default().fg(theme.cursor_bar_color())),
                Span::styled(
                    checked,
                    Style::default()
                        .fg(theme
                            .mode_normal_style()
                            .bg
                            .unwrap_or(theme.cursor_bar_color()))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    if has_update { "↑ " } else { "  " },
                    if has_update {
                        Style::default()
                            .fg(theme.border_focused_color())
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(
                    format!("{name:<width$}", width = available_width.saturating_sub(2)),
                    theme.badge_style_for_manager(package.source.name()),
                ),
                Span::styled(
                    format!(" {size:>9}"),
                    theme.size_style_for_bytes(package.size_bytes),
                ),
            ];
            ListItem::new(Line::from(spans)).style(if is_selected {
                theme.selection_style()
            } else {
                Style::default()
            })
        })
        .collect::<Vec<_>>();
    let mut state =
        ListState::default().with_selected((!app.filtered.is_empty()).then_some(app.package_index));
    frame.render_stateful_widget(
        List::new(items).style(Style::default().fg(theme.fg_color()).bg(panel_bg)),
        list_area,
        &mut state,
    );
    if let Some((rule_area, deps_area)) = deps_area {
        frame.render_widget(
            Paragraph::new("─".repeat(rule_area.width as usize))
                .style(Style::default().fg(theme.border_color()).bg(panel_bg)),
            rule_area,
        );
        render_package_dependencies(frame, deps_area, app, theme, panel_bg);
    }
}

fn render_package_dependencies(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    theme: &ThemeColors,
    background: ratatui::style::Color,
) {
    let Some(package) = app.selected_package() else {
        return;
    };
    let heading = Line::from(vec![
        Span::styled(
            " DEPENDENCIES ",
            Style::default()
                .fg(theme.border_focused_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} ", package.dependencies.len()),
            Style::default().fg(theme.border_color()),
        ),
        Span::styled("(i hide)", Style::default().fg(theme.border_color())),
    ]);
    let lines = package
        .dependencies
        .iter()
        .enumerate()
        .map(|(index, dependency)| {
            let selected = index == app.dependency_index;
            let marker = if selected { "▍ " } else { "  " };
            let installed = app.selected_dependency_package().is_some();
            let suffix = if installed { "" } else { " (not in inventory)" };
            let line = Line::from(vec![
                Span::styled(
                    marker,
                    Style::default().fg(if selected {
                        theme.cursor_bar_color()
                    } else {
                        theme.border_color()
                    }),
                ),
                Span::raw(format!("{dependency}{suffix}")),
            ]);
            ListItem::new(line).style(if selected && app.focus == Focus::PackageDependencies {
                theme.selection_style()
            } else {
                Style::default().fg(theme.fg_color()).bg(background)
            })
        })
        .collect::<Vec<_>>();
    let visible = area.height.saturating_sub(1) as usize;
    let start = app
        .dependency_index
        .saturating_sub(visible.saturating_sub(1));
    let items = lines
        .into_iter()
        .skip(start)
        .take(visible)
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(heading).style(Style::default().fg(theme.fg_color()).bg(background)),
        Rect::new(area.x, area.y, area.width, 1),
    );
    frame.render_widget(
        List::new(items).style(Style::default().fg(theme.fg_color()).bg(background)),
        Rect::new(
            area.x,
            area.y.saturating_add(1),
            area.width,
            area.height.saturating_sub(1),
        ),
    );
}

fn render_details(frame: &mut Frame, area: Rect, app: &App, theme: &ThemeColors) {
    let panel_bg = theme.panel_details_bg_color();
    let title = if app.focus == Focus::Details {
        " DETAILS "
    } else {
        " Details "
    };
    let content_area = draw_panel(
        frame,
        area,
        title,
        app.focus == Focus::Details,
        app,
        theme,
        panel_bg,
    );
    if let Some(console) = app.uninstall_console.as_ref() {
        render_uninstall_console(
            frame,
            content_area,
            console,
            app.console_scroll,
            theme,
            panel_bg,
        );
        return;
    }
    let Some(package) = app.selected_detail_package() else {
        let missing_dependency = app
            .show_package_dependencies
            .then(|| app.selected_dependency_name())
            .flatten();
        let message = if let Some(dependency) = missing_dependency {
            format!("Dependency '{dependency}' is not in the installed inventory")
        } else {
            "No package selected".to_owned()
        };
        frame.render_widget(
            Paragraph::new(message)
                .style(Style::default().fg(theme.fg_color()).bg(panel_bg))
                .alignment(Alignment::Center),
            content_area,
        );
        return;
    };
    let mut lines = vec![
        Line::from(Span::styled(
            format!("{} {}", package.name, package.version),
            Style::default()
                .fg(theme.border_focused_color())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        detail_line("Source", &package.source.to_string(), theme),
        detail_line(
            "Install type",
            &format!("{:?}", package.install_reason).to_lowercase(),
            theme,
        ),
        detail_line(
            "Size",
            &package
                .size_bytes
                .map(|bytes| format_bytes(Some(bytes)))
                .unwrap_or_else(|| "Unknown".into()),
            theme,
        ),
    ];
    if app.focus == Focus::PackageDependencies {
        lines.insert(
            2,
            detail_line(
                "Dependency",
                app.selected_dependency_name().unwrap_or(""),
                theme,
            ),
        );
    }
    if let Some(date) = &package.install_date {
        lines.push(detail_line("Installed", date, theme));
    }
    if let Some(project_id) = package.manager_id.as_deref()
        && let Some((_, root)) = project_id
            .strip_prefix("project:")
            .and_then(|rest| rest.split_once(':'))
    {
        lines.push(detail_line("Project", root, theme));
    }
    if let Some(latest) = &package.latest_version {
        lines.push(detail_line("Available", latest, theme));
    }
    if let Some(license) = &package.license {
        lines.push(detail_line("License", license, theme));
    }
    if let Some(url) = &package.url {
        lines.push(detail_line("URL", url, theme));
    }
    if !package.binaries.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Binaries",
            Style::default()
                .fg(theme.border_focused_color())
                .add_modifier(Modifier::BOLD),
        )));
        lines.extend(
            package
                .binaries
                .iter()
                .map(|v| Line::from(format!("  {v}"))),
        );
    }
    if let Some(description) = &package.description {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Description",
            Style::default()
                .fg(theme.border_focused_color())
                .add_modifier(Modifier::BOLD),
        )));
        lines.extend(
            description
                .lines()
                .map(|line| Line::from(format!("{line}"))),
        );
    }
    if !package.dependencies.is_empty() && !app.show_package_dependencies {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Dependencies",
            Style::default()
                .fg(theme.border_focused_color())
                .add_modifier(Modifier::BOLD),
        )));
        lines.extend(
            package
                .dependencies
                .iter()
                .map(|dep| Line::from(format!("  {dep}"))),
        );
    }
    if !package.files.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Files",
            Style::default()
                .fg(theme.border_focused_color())
                .add_modifier(Modifier::BOLD),
        )));
        lines.extend(
            package
                .files
                .iter()
                .take(30)
                .map(|path| Line::from(format!("  {path}"))),
        );
    }
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .scroll((app.detail_scroll, 0))
            .style(Style::default().fg(theme.fg_color()).bg(panel_bg)),
        content_area,
    );
}

fn detail_line(label: &str, value: &str, theme: &ThemeColors) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label:<13}"),
            Style::default().fg(theme.border_color()),
        ),
        Span::raw(value.to_owned()),
    ])
}

fn render_uninstall_console(
    frame: &mut Frame,
    area: Rect,
    console: &UninstallConsole,
    scroll: usize,
    theme: &ThemeColors,
    background: ratatui::style::Color,
) {
    let state = if console.running {
        "running"
    } else if console.exit_code == Some(0) {
        "success"
    } else {
        "failed"
    };
    let title = Line::from(vec![
        Span::styled(
            format!(" {} ", console.operation.label()),
            Style::default()
                .fg(theme.border_focused_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} · {}", console.package, state),
            Style::default().fg(theme.fg_color()),
        ),
    ]);
    let lines = console
        .lines
        .iter()
        .map(|line| {
            let style = if line.starts_with("$ ") {
                Style::default()
                    .fg(theme.border_focused_color())
                    .add_modifier(Modifier::BOLD)
            } else if line.contains("error") || line.contains("failed") || line.contains("Error") {
                Style::default().fg(ratatui::style::Color::Rgb(0xea, 0x69, 0x62))
            } else {
                Style::default().fg(theme.fg_color())
            };
            Line::from(Span::styled(line.clone(), style))
        })
        .collect::<Vec<_>>();
    let inner = Rect::new(area.x, area.y, area.width, area.height);
    if inner.height > 0 {
        let max_scroll = console
            .lines
            .len()
            .saturating_sub(area.height.saturating_sub(1) as usize);
        let scroll = if scroll == usize::MAX {
            max_scroll
        } else {
            scroll.min(max_scroll)
        };
        frame.render_widget(
            Paragraph::new(title).style(Style::default().bg(background)),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .scroll((scroll.min(u16::MAX as usize) as u16, 0))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(theme.fg_color()).bg(background)),
            Rect::new(
                inner.x,
                inner.y.saturating_add(1),
                inner.width,
                inner.height.saturating_sub(1),
            ),
        );
    }
}

fn render_operation_confirm(
    frame: &mut Frame,
    area: Rect,
    operation: OperationKind,
    packages: &[UnifiedPackage],
    command: &str,
    theme: &ThemeColors,
    border: ratatui::widgets::BorderType,
) {
    let popup = centered(area, 78, 20);
    frame.render_widget(Clear, popup);
    let names = packages
        .iter()
        .map(|package| format!("{} ({})", package.name, package.source))
        .collect::<Vec<_>>()
        .join("\n");
    let content = match operation {
        OperationKind::Uninstall => format!(
            "Remove {} package(s)?\n{}\n\nRemoving system packages may require administrator privileges. Omni runs as your user and asks for sudo only for this operation. Pacman removes only named targets and refuses removal if installed packages require them.\nCustom managers may run arbitrary configured shell actions.\n\nCommands:\n{}\n\n[y/Enter] confirm   [n/Esc] cancel",
            packages.len(),
            names,
            command
        ),
        OperationKind::Update => format!(
            "Update {} package(s)?\n{}\n\nOnly packages with a detected update are selected. Arch does not support partial upgrades: pacman -Syu may also upgrade every other outdated system package. AUR updates can run package build scripts as your user.\n\nCommands:\n{}\n\n[y/Enter] confirm   [n/Esc] cancel",
            packages.len(),
            names,
            command
        ),
        OperationKind::Add => format!(
            "Add project dependency?\n{}\n\nCommand:\n{}",
            names, command
        ),
    };
    frame.render_widget(
        Paragraph::new(content)
            .scroll((0, 0))
            .wrap(Wrap { trim: true })
            .block(popup_block(
                match operation {
                    OperationKind::Update => " Confirm update ",
                    OperationKind::Add => " Add dependency ",
                    OperationKind::Uninstall => " Confirm uninstall ",
                },
                theme,
                border,
            )),
        popup,
    );
}

fn draw_panel(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    focused: bool,
    app: &App,
    theme: &ThemeColors,
    background: ratatui::style::Color,
) -> Rect {
    let title_style = if focused {
        Style::default()
            .fg(theme.border_focused_color())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.fg_color())
    };
    let panel_style = Style::default().fg(theme.fg_color()).bg(background);
    frame.render_widget(Block::default().style(panel_style), area);

    if !app.config.layout.has_panel_borders() {
        let show_focus_bar = app.config.layout.border_style.eq_ignore_ascii_case("none");
        let focus_gutter = u16::from(show_focus_bar);
        let horizontal_padding = app.config.layout.padding;
        let content_x = area
            .x
            .saturating_add(horizontal_padding)
            .saturating_add(focus_gutter);
        let content_width = area.width.saturating_sub(
            horizontal_padding
                .saturating_mul(2)
                .saturating_add(focus_gutter),
        );
        if show_focus_bar && focused && area.width > 0 {
            let gutter = Rect::new(area.x, area.y, 1, area.height);
            let marker_style = Style::default().fg(theme.cursor_bar_color()).bg(background);
            let marker_lines = (0..area.height)
                .map(|_| Line::from(Span::styled("▍", marker_style)))
                .collect::<Vec<_>>();
            frame.render_widget(Paragraph::new(marker_lines), gutter);
        }
        if area.height > 0 {
            let heading = Rect::new(content_x, area.y, content_width, 1);
            frame.render_widget(
                Paragraph::new(Span::styled(title, title_style)).style(panel_style),
                heading,
            );
        }
        return Rect::new(
            content_x,
            area.y.saturating_add(1),
            content_width,
            area.height.saturating_sub(1),
        );
    }

    let mut block = Block::default()
        .title(Span::styled(title, title_style))
        .borders(Borders::ALL)
        .border_type(app.config.layout.border_type())
        .border_style(Style::default().fg(if focused {
            theme.border_focused_color()
        } else {
            theme.border_color()
        }))
        .style(panel_style);
    if app.config.layout.padding > 0 {
        block = block.padding(ratatui::widgets::Padding::horizontal(
            app.config.layout.padding,
        ));
    }
    let content = block.inner(area);
    frame.render_widget(block, area);
    content
}

fn render_dividers(frame: &mut Frame, dividers: &[Rect], theme: &ThemeColors) {
    for area in dividers {
        let lines = (0..area.height)
            .map(|_| Line::from("│"))
            .collect::<Vec<_>>();
        frame.render_widget(
            Paragraph::new(lines).style(Style::default().fg(theme.border_color())),
            *area,
        );
    }
}

fn popup_block<'a>(
    title: &'a str,
    theme: &ThemeColors,
    border: ratatui::widgets::BorderType,
) -> Block<'a> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(border)
        .border_style(Style::default().fg(theme.border_focused_color()))
        .style(Style::default().bg(theme.bg_color()).fg(theme.fg_color()))
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width.saturating_sub(2)).max(1);
    let height = height.min(area.height.saturating_sub(2)).max(1);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

fn truncate(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
}

#[cfg(test)]
mod uninstall_ui_tests {
    use super::dependency_package_name;

    #[test]
    fn resolves_version_constrained_dependency_names() {
        assert_eq!(dependency_package_name("glibc>=2.38"), "glibc");
        assert_eq!(dependency_package_name("openssl < 4"), "openssl");
        assert_eq!(dependency_package_name("zlib"), "zlib");
    }
}

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

/// Draw an in-app password prompt in the status row. The password is masked and
/// is only sent to sudo over the child's stdin after confirmation.
fn render_sudo_password_prompt(frame: &mut Frame, area: Rect, length: usize, theme: &ThemeColors) {
    frame.render_widget(Clear, area);
    let bullets = "•".repeat(length.min(area.width as usize / 2));
    let prompt = Line::from(vec![
        Span::styled(
            " sudo ",
            Style::default()
                .fg(theme.bg_color())
                .bg(theme.border_focused_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" password: "),
        Span::raw(bullets),
        Span::styled(
            "▏  Enter: confirm  Esc: cancel",
            Style::default().fg(theme.border_color()),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(prompt).style(Style::default().fg(theme.fg_color()).bg(theme.bg_color())),
        area,
    );
}

fn render_project_add_prompt(
    frame: &mut Frame,
    area: Rect,
    input: &str,
    project_label: &str,
    theme: &ThemeColors,
) {
    let prompt = Line::from(vec![
        Span::styled(
            " ADD ",
            Style::default()
                .fg(theme.bg_color())
                .bg(theme.border_focused_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {project_label} · "),
            Style::default().fg(theme.border_color()),
        ),
        Span::raw(input),
        Span::styled("▏", Style::default().fg(theme.border_focused_color())),
        Span::styled(
            "  Enter: run  Esc: cancel",
            Style::default().fg(theme.border_color()),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(prompt).style(Style::default().fg(theme.fg_color()).bg(theme.bg_color())),
        area,
    );
}

impl TerminalGuard {
    fn enter() -> Result<Self> {
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).context("enter alternate screen")?;
        if let Err(error) = terminal::enable_raw_mode() {
            let _ = execute!(stdout, LeaveAlternateScreen);
            return Err(error).context("enable raw terminal mode");
        }
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = match Terminal::new(backend) {
            Ok(terminal) => terminal,
            Err(error) => {
                let mut stdout = io::stdout();
                let _ = terminal::disable_raw_mode();
                let _ = execute!(stdout, LeaveAlternateScreen);
                return Err(error).context("initialize terminal UI");
            }
        };
        if let Err(error) = terminal.hide_cursor() {
            let _ = terminal::disable_raw_mode();
            let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
            return Err(error).context("hide terminal cursor");
        }
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = self.terminal.show_cursor();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
    }
}
