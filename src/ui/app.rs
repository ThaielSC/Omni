use std::io::{self, BufRead, BufReader, Stdout};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
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

use crate::config::{
    Action, OmniConfig, ThemeColors, default_config_path, default_keymap_path,
    default_managers_path, default_theme_path, parse_keymap_toml, parse_managers_toml,
    parse_theme_toml,
};
use crate::model::{FilterTab, InstallReason, SortMode, UnifiedPackage};
use crate::provider::{
    PackageManagerProvider, build_uninstall_command, cargo::CargoProvider, custom::CustomManager,
    flatpak::FlatpakProvider, pacman::PacmanProvider, pip::PipProvider, uv::UvProvider,
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
}

#[derive(Debug)]
enum UiEvent {
    Packages(Vec<UnifiedPackage>),
    UninstallOutput(String),
    UninstallFinished(Option<i32>),
}

#[derive(Debug, Clone)]
struct UninstallConsole {
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
    package_index: usize,
    detail_scroll: u16,
    sort: SortMode,
    show_dependencies: bool,
    query: String,
    searching: bool,
    overlay: Option<Overlay>,
    help_filter: String,
    help_filtering: bool,
    loading: bool,
    reload_after_loading: bool,
    notice: Option<(String, Instant)>,
    reload_tx: Sender<UiEvent>,
    show_package_dependencies: bool,
    dependency_index: usize,
    uninstall_target: Option<UnifiedPackage>,
    uninstall_command: Option<crate::provider::UninstallCommand>,
    uninstall_console: Option<UninstallConsole>,
    console_scroll: usize,
    sudo_prompt_requested: bool,
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
            package_index: 0,
            detail_scroll: 0,
            sort: SortMode::SizeDesc,
            show_dependencies: false,
            query: String::new(),
            searching: false,
            overlay: None,
            help_filter: String::new(),
            help_filtering: false,
            loading: true,
            reload_after_loading: false,
            notice: None,
            reload_tx,
            show_package_dependencies: false,
            dependency_index: 0,
            uninstall_target: None,
            uninstall_command: None,
            uninstall_console: None,
            console_scroll: 0,
            sudo_prompt_requested: false,
        }
    }

    fn update_filter(&mut self) {
        let matcher = SkimMatcherV2::default();
        self.filtered = self
            .packages
            .iter()
            .enumerate()
            .filter(|(_, package)| self.tab.matches(package.source))
            .filter(|(_, package)| {
                self.show_dependencies || package.install_reason != InstallReason::Dependency
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

    fn handle_key(&mut self, key: KeyEvent) -> bool {
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
                KeyCode::Esc | KeyCode::Char('n') if overlay == Overlay::ConfirmUninstall => {
                    self.overlay = None;
                    self.uninstall_target = None;
                    self.uninstall_command = None;
                }
                KeyCode::Enter | KeyCode::Char('y') if overlay == Overlay::ConfirmUninstall => {
                    self.overlay = None;
                    self.start_uninstall();
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
            self.uninstall_console = None;
            self.detail_scroll = 0;
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
            Some(Action::Reload) => self.start_reload(),
            Some(Action::Uninstall) => self.prepare_uninstall(),
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
        self.package_index = 0;
        self.focus = Focus::Packages;
        self.update_filter();
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
        self.focus = panes[next as usize];
    }

    fn move_selection(&mut self, amount: isize) {
        match self.focus {
            Focus::Managers => {
                let next = (self.manager_index as isize + amount).clamp(0, 5);
                self.manager_index = next as usize;
                if let Some(tab) = FilterTab::from_index(self.manager_index) {
                    self.tab = tab;
                    self.package_index = 0;
                    self.update_filter();
                }
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
            Focus::Managers => self.manager_index = if bottom { 5 } else { 0 },
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
        if self.focus == Focus::Managers
            && let Some(tab) = FilterTab::from_index(self.manager_index)
        {
            self.tab = tab;
            self.package_index = 0;
            self.update_filter();
        }
    }

    fn start_reload(&mut self) {
        if self.loading {
            return;
        }
        self.loading = true;
        // The provider scan is intentionally detached from the event loop; the
        // UI remains responsive and receives its result through the channel.
        let tx = self.reload_tx.clone();
        let custom = self.config.custom_managers.clone();
        thread::spawn(move || {
            let _ = tx.send(UiEvent::Packages(scan_packages(custom)));
        });
    }

    fn prepare_uninstall(&mut self) {
        let target = if matches!(self.focus, Focus::PackageDependencies | Focus::Details)
            && self.show_package_dependencies
        {
            self.selected_dependency_package().cloned()
        } else {
            self.selected_package().cloned()
        };
        let Some(target) = target else {
            self.set_notice("Dependency is not present in the installed package inventory");
            return;
        };
        match build_uninstall_command(&target, &self.config.behavior, &self.config.custom_managers)
        {
            Ok(command) => {
                self.uninstall_target = Some(target);
                self.uninstall_command = Some(command);
                if self.config.behavior.confirm_uninstall {
                    self.overlay = Some(Overlay::ConfirmUninstall);
                } else {
                    self.start_uninstall();
                }
            }
            Err(error) => self.set_notice(error),
        }
    }

    fn start_uninstall(&mut self) {
        if self
            .uninstall_command
            .as_ref()
            .is_some_and(|command| command.sudo_program().is_some())
        {
            self.sudo_prompt_requested = true;
            return;
        }
        self.launch_uninstall();
    }

    fn launch_uninstall(&mut self) {
        let (Some(target), Some(command)) =
            (self.uninstall_target.take(), self.uninstall_command.take())
        else {
            return;
        };
        let tx = self.reload_tx.clone();
        let display = command.display.clone();
        self.uninstall_console = Some(UninstallConsole {
            package: target.name.clone(),
            lines: vec![format!("$ {display}"), String::new()],
            running: true,
            exit_code: None,
        });
        self.console_scroll = usize::MAX;
        self.focus = Focus::Details;
        thread::spawn(move || {
            let mut process = command.command();
            process
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
            let mut child = match process.spawn() {
                Ok(child) => child,
                Err(error) => {
                    let _ = tx.send(UiEvent::UninstallOutput(format!(
                        "Failed to start command: {error}"
                    )));
                    let _ = tx.send(UiEvent::UninstallFinished(None));
                    return;
                }
            };
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
            let _ = tx.send(UiEvent::UninstallFinished(code));
        });
    }

    fn pending_sudo_program(&self) -> Option<&str> {
        self.uninstall_command.as_ref()?.sudo_program()
    }

    fn sudo_authorization_finished(&mut self, authorized: bool, error: Option<String>) {
        if authorized {
            self.launch_uninstall();
        } else {
            self.uninstall_target = None;
            self.uninstall_command = None;
            self.set_notice(error.unwrap_or_else(|| {
                "sudo authorization cancelled; uninstall was not run".to_owned()
            }));
        }
    }

    fn apply_config(&mut self, config: OmniConfig, managers_changed: bool) {
        self.config = config;
        if !self.config.layout.show_managers && self.focus == Focus::Managers {
            self.focus = Focus::Packages;
        }
        if !self.config.layout.show_details && self.focus == Focus::Details {
            self.focus = Focus::Packages;
        }
        self.set_notice("Configuration reloaded");
        if managers_changed {
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

    fn apply_ui_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::Packages(mut packages) => {
                self.sort.sort_packages(&mut packages);
                self.packages = packages;
                self.loading = false;
                self.update_filter();
                if self.reload_after_loading {
                    self.reload_after_loading = false;
                    self.start_reload();
                }
            }
            UiEvent::UninstallOutput(line) => {
                if let Some(console) = self.uninstall_console.as_mut() {
                    console.lines.push(line);
                    if console.lines.len() > 2_000 {
                        let excess = console.lines.len() - 2_000;
                        console.lines.drain(..excess);
                    }
                    self.console_scroll = usize::MAX;
                }
            }
            UiEvent::UninstallFinished(code) => {
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
                    self.set_notice("Uninstall completed; refreshing package inventory");
                    self.start_reload();
                } else {
                    self.set_notice("Uninstall failed; see output in Details");
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
                let _ = tx.send(UiEvent::UninstallOutput(line));
            }
            Err(error) => {
                let _ = tx.send(UiEvent::UninstallOutput(format!(
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
    thread::spawn(move || {
        let _ = tx.send(UiEvent::Packages(scan_packages(custom)));
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
        if let Some(result) = watcher.poll() {
            match result {
                Ok((config, managers_changed)) => app.apply_config(config, managers_changed),
                Err(error) => app.set_notice(format!("Config error: {error}")),
            }
        }
        terminal.draw(|frame| render(frame, app))?;
        if event::poll(Duration::from_millis(80))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Release {
                    continue;
                }
                if app.handle_key(key) {
                    break;
                }
                if app.sudo_prompt_requested {
                    app.sudo_prompt_requested = false;
                    let sudo = app.pending_sudo_program().unwrap_or("sudo").to_owned();
                    match authorize_sudo_in_terminal(terminal, &sudo) {
                        Ok(status) => app.sudo_authorization_finished(status.success(), None),
                        // Unwind the terminal guard if prompt launch/restoration
                        // fails; drawing again could corrupt a non-raw screen.
                        Err(error) => return Err(error),
                    }
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

fn scan_packages(custom_configs: Vec<crate::config::CustomManagerConfig>) -> Vec<UnifiedPackage> {
    let providers: Vec<Box<dyn PackageManagerProvider>> = vec![
        Box::new(PacmanProvider),
        Box::new(YayProvider),
        Box::new(FlatpakProvider),
        Box::new(CargoProvider),
        Box::new(UvProvider),
        Box::new(PipProvider),
    ];
    let mut packages = Vec::new();
    for provider in providers {
        if provider.is_available()
            && let Ok(mut found) = provider.list_packages()
        {
            packages.append(&mut found);
        }
    }
    for provider in CustomManager::from_configs(custom_configs) {
        if provider.is_available()
            && let Ok(mut found) = provider.list_packages()
        {
            packages.append(&mut found);
        }
    }
    packages
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
        app.tab,
        app.package_index,
        app.filtered.len(),
        total_size,
        app.sort,
        app.show_dependencies,
        app.notice
            .as_ref()
            .filter(|(_, expires)| *expires > Instant::now())
            .map(|(message, _)| message.as_str()),
    );
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
            help::render(
                frame,
                frame.area(),
                theme,
                app.config.layout.border_type(),
                &dependency_key,
                &package_dependency_key,
                &app.help_filter,
                app.help_filtering,
            )
        }
        Some(Overlay::ConfirmUninstall) => {
            if let (Some(package), Some(command)) = (&app.uninstall_target, &app.uninstall_command)
            {
                render_uninstall_confirm(
                    frame,
                    frame.area(),
                    package,
                    &command.display,
                    theme,
                    app.config.layout.border_type(),
                );
            }
        }
        None => {}
    }
    if app.loading {
        let area = centered(frame.area(), 44, 17);
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new("Scanning installed packages…")
                .block(popup_block(
                    " Loading ",
                    theme,
                    app.config.layout.border_type(),
                ))
                .alignment(Alignment::Center),
            area,
        );
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
    let selected_style = theme.selection_style().add_modifier(Modifier::BOLD);
    let items = FilterTab::ALL
        .iter()
        .enumerate()
        .map(|(index, tab)| {
            let selected = index == app.manager_index;
            let line = Line::from(vec![
                Span::styled(
                    if selected { "▍" } else { " " },
                    if selected {
                        Style::default().fg(theme.cursor_bar_color())
                    } else {
                        Style::default()
                    },
                ),
                Span::raw(format!(" {:<9} {:>5}", tab.display_name(), counts[index])),
            ]);
            ListItem::new(line).style(if selected {
                selected_style
            } else {
                Style::default().fg(theme.fg_color())
            })
        })
        .collect::<Vec<_>>();
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
            let size = package
                .size_bytes
                .map(|bytes| format_bytes(Some(bytes)))
                .unwrap_or_else(|| "—".into());
            let available_width = content_area.width.saturating_sub(17) as usize;
            let name = truncate(&package.name, available_width);
            let spans = vec![
                Span::styled(marker, Style::default().fg(theme.cursor_bar_color())),
                Span::styled(
                    format!("{name:<width$}", width = available_width),
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
        } else if app.loading {
            "Scanning installed packages…".to_owned()
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
            " UNINSTALL ",
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

fn render_uninstall_confirm(
    frame: &mut Frame,
    area: Rect,
    package: &UnifiedPackage,
    command: &str,
    theme: &ThemeColors,
    border: ratatui::widgets::BorderType,
) {
    let popup = centered(area, 74, 13);
    frame.render_widget(Clear, popup);
    let content = format!(
        "Remove {} ({})?\n\nReview the command: custom managers may run arbitrary configured shell actions. Pacman targets only this package and refuses removal if another installed package requires it.\nFor sudo, Omni briefly restores the terminal and lets sudo prompt; Omni never reads or stores the password.\n\nCommand:\n{}\n\n[y/Enter] confirm   [n/Esc] cancel",
        package.name, package.source, command
    );
    frame.render_widget(
        Paragraph::new(content)
            .wrap(Wrap { trim: true })
            .block(popup_block(" Confirm uninstall ", theme, border)),
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

/// Let sudo interact with the real terminal without exposing password input to
/// Omni or leaving the TUI in raw mode. Restore the alternate screen even when
/// sudo exits unsuccessfully; the outer terminal guard remains a final fallback.
fn authorize_sudo_in_terminal(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    sudo_program: &str,
) -> Result<ExitStatus> {
    terminal
        .show_cursor()
        .context("show cursor for sudo prompt")?;
    terminal::disable_raw_mode().context("leave raw mode for sudo prompt")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("leave alternate screen for sudo prompt")?;

    let status = Command::new(sudo_program).arg("-v").status();

    execute!(terminal.backend_mut(), EnterAlternateScreen)
        .context("restore alternate screen after sudo prompt")?;
    terminal::enable_raw_mode().context("restore raw mode after sudo prompt")?;
    terminal
        .hide_cursor()
        .context("hide cursor after sudo prompt")?;
    terminal.clear().context("redraw TUI after sudo prompt")?;
    Ok(status.context("run sudo credential prompt")?)
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
