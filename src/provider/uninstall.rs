//! Safe command construction for installed-package operations.
//!
//! Native providers are invoked directly with argument vectors. Custom manager
//! commands are explicitly shell-based because their TOML command templates may
//! contain normal shell syntax; package fields are POSIX-shell escaped before
//! being substituted into those templates.

use crate::config::{BehaviorConfig, CustomManagerConfig, ProjectManagersConfig};
use crate::model::{PackageSource, UnifiedPackage};
use std::process::Command;

/// A fully rendered uninstall operation, suitable for confirmation UI and execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UninstallCommand {
    /// Human-readable command preview shown before execution.
    pub display: String,
    program: String,
    args: Vec<String>,
    sudo_noninteractive_index: Option<usize>,
    stdin_password: bool,
    current_dir: Option<std::path::PathBuf>,
}

impl UninstallCommand {
    fn direct(
        program: impl Into<String>,
        args: Vec<String>,
        display: String,
        sudo_noninteractive_index: Option<usize>,
    ) -> Self {
        Self {
            display,
            program: program.into(),
            args,
            sudo_noninteractive_index,
            stdin_password: sudo_noninteractive_index.is_some(),
            current_dir: None,
        }
    }

    pub fn custom_update(
        program: String,
        args: Vec<String>,
        behavior: &BehaviorConfig,
        needs_privileges: bool,
    ) -> Result<Self, String> {
        let (program, args, display, sudo_index) =
            privileged_command(&program, args, behavior, needs_privileges)?;
        Ok(Self::direct(program, args, display, sudo_index))
    }

    /// Mark a command that delegates sudo authentication to a child process
    /// (for example, yay calling sudo internally) as consuming a password on
    /// stdin.
    pub fn with_stdin_password(mut self) -> Self {
        self.stdin_password = true;
        self
    }

    /// Whether the operation needs the in-app password prompt.
    pub fn requires_stdin_password(&self) -> bool {
        self.stdin_password
    }

    /// Build a fresh process command for this operation.
    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        if let Some(directory) = &self.current_dir {
            command.current_dir(directory);
        }
        command
    }

    /// Executable path/name used by the command.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// Arguments passed to the executable (or shell).
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Return the sudo executable when this operation is configured to use it.
    pub fn sudo_program(&self) -> Option<&str> {
        (std::path::Path::new(&self.program).file_name()?.to_str()? == "sudo")
            .then_some(self.program.as_str())
    }

    /// Build the sudo form that reads its password from stdin. The caller must
    /// keep the password out of args, environment variables, and logs.
    pub fn command_with_stdin_password(&self) -> Command {
        let mut command = Command::new(&self.program);
        if let Some(directory) = &self.current_dir {
            command.current_dir(directory);
        }
        let Some(index) = self.sudo_noninteractive_index else {
            command.args(&self.args);
            return command;
        };
        let mut args = self.args.clone();
        args.remove(index);
        args.splice(
            index..index,
            [
                "-k".to_owned(),
                "-S".to_owned(),
                "-p".to_owned(),
                String::new(),
            ],
        );
        command.args(args);
        command
    }

    fn in_directory(mut self, directory: std::path::PathBuf) -> Self {
        self.current_dir = Some(directory);
        self
    }
}

/// Construct an uninstall command for a package from a supported native or
/// user-defined manager. This function only prepares a command; it never runs it.
pub fn build_uninstall_command(
    package: &UnifiedPackage,
    behavior: &BehaviorConfig,
    managers: &[CustomManagerConfig],
) -> Result<UninstallCommand, String> {
    build_uninstall_command_with_projects(
        package,
        behavior,
        managers,
        &ProjectManagersConfig::default(),
    )
}

pub fn build_uninstall_command_with_projects(
    package: &UnifiedPackage,
    behavior: &BehaviorConfig,
    managers: &[CustomManagerConfig],
    projects: &ProjectManagersConfig,
) -> Result<UninstallCommand, String> {
    let name = validate_value("package name", &package.name)?;
    let version = validate_value("package version", &package.version)?;

    if let Some(manager_id) = package.manager_id.as_deref() {
        if let Some((ecosystem, root)) = parse_project_manager_id(manager_id) {
            return build_project_command(ecosystem, root, name, false, projects);
        }
        let manager = managers
            .iter()
            .find(|manager| manager.id == manager_id)
            .ok_or_else(|| format!("Custom package manager '{manager_id}' is not configured"))?;
        return build_custom_command(manager, name, version, behavior);
    }

    match package.source {
        PackageSource::Pacman | PackageSource::Aur => {
            let args = vec![
                // Remove only the selected package. `-s` may cascade into
                // dependencies that the confirmation dialog cannot enumerate.
                "-R".to_owned(),
                "--noconfirm".to_owned(),
                "--".to_owned(),
                name.to_owned(),
            ];
            let (program, args, display, sudo_index) =
                privileged_command("pacman", args, behavior, true)?;
            Ok(UninstallCommand::direct(program, args, display, sudo_index))
        }
        PackageSource::Flatpak => {
            let args = vec![
                "uninstall".to_owned(),
                "--noninteractive".to_owned(),
                "--".to_owned(),
                name.to_owned(),
            ];
            let (program, args, display, sudo_index) =
                privileged_command("flatpak", args, behavior, false)?;
            Ok(UninstallCommand::direct(program, args, display, sudo_index))
        }
        PackageSource::Cargo => {
            let args = vec!["uninstall".to_owned(), name.to_owned()];
            let (program, args, display, sudo_index) =
                privileged_command("cargo", args, behavior, false)?;
            Ok(UninstallCommand::direct(program, args, display, sudo_index))
        }
        PackageSource::Uv => {
            let args = vec!["tool".to_owned(), "uninstall".to_owned(), name.to_owned()];
            let (program, args, display, sudo_index) =
                privileged_command("uv", args, behavior, false)?;
            Ok(UninstallCommand::direct(program, args, display, sudo_index))
        }
        PackageSource::Pip => {
            let args = vec!["uninstall".to_owned(), "-y".to_owned(), name.to_owned()];
            let (program, args, display, sudo_index) =
                privileged_command("pip", args, behavior, false)?;
            Ok(UninstallCommand::direct(program, args, display, sudo_index))
        }
        PackageSource::Project => {
            Err("Project packages require a project manager identity".to_owned())
        }
    }
}

/// Query Pacman for orphaned dependency packages. This is read-only; removal is
/// still initiated one package at a time through the normal confirmation flow.
pub fn list_pacman_orphans() -> Result<Vec<String>, String> {
    let output = Command::new("pacman")
        .args(["-Qtdq"])
        .output()
        .map_err(|error| format!("could not start pacman orphan query: {error}"))?;
    if !output.status.success() && output.status.code() != Some(1) {
        return Err(format!(
            "pacman orphan query failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect())
}

#[cfg(test)]
mod update_command_tests {
    use super::*;

    #[test]
    fn project_update_builds_manager_specific_safe_commands() {
        let config = ProjectManagersConfig::default();
        let names = vec!["serde".to_owned(), "tokio".to_owned()];
        let cargo = build_project_update_command(
            "project:cargo:/tmp/my-app",
            std::path::Path::new("/tmp/my-app"),
            &names,
            &config,
        )
        .unwrap();
        assert_eq!(cargo.program(), "cargo");
        assert_eq!(cargo.args(), ["update", "-p", "serde", "-p", "tokio"]);
        let npm = build_project_update_command(
            "project:npm:/tmp/my-app",
            std::path::Path::new("/tmp/my-app"),
            &names[..1],
            &config,
        )
        .unwrap();
        assert_eq!(npm.args(), ["update", "serde"]);
    }

    #[test]
    fn project_update_rejects_option_injection() {
        let error = build_project_update_command(
            "project:bun:/tmp/app",
            std::path::Path::new("/tmp/app"),
            &["--force".to_owned()],
            &ProjectManagersConfig::default(),
        )
        .unwrap_err();
        assert!(error.contains("must not begin"));
    }
}

/// Build the project dependency-add operation using a direct argv (never a
/// shell). It runs with the project root as its working directory.
pub fn build_project_add_command(
    manager_id: &str,
    project_root: &std::path::Path,
    package_name: &str,
    config: &ProjectManagersConfig,
) -> Result<UninstallCommand, String> {
    let (ecosystem, _) = parse_project_manager_id(manager_id)
        .ok_or_else(|| "Invalid project manager identity".to_owned())?;
    if !config.supports(ecosystem) {
        return Err(format!(
            "Project manager '{ecosystem}' is disabled in config.toml"
        ));
    }
    build_project_command(ecosystem, project_root, package_name, true, config)
}

/// Build project-local update commands. Cargo and Node package managers treat
/// project updates as lockfile operations, so a selected package narrows Cargo
/// with `-p` while npm/Bun update only the named direct dependency.
pub fn build_project_update_command(
    manager_id: &str,
    project_root: &std::path::Path,
    package_names: &[String],
    config: &ProjectManagersConfig,
) -> Result<UninstallCommand, String> {
    let (ecosystem, _) = parse_project_manager_id(manager_id)
        .ok_or_else(|| "Invalid project manager identity".to_owned())?;
    if !config.supports(ecosystem) {
        return Err(format!(
            "Project manager '{ecosystem}' is disabled in config.toml"
        ));
    }
    if package_names.is_empty() {
        return Err("No project packages selected".into());
    }
    for name in package_names {
        validate_value("project package name", name)?;
        if name.starts_with('-')
            || name.chars().any(char::is_whitespace)
            || name.chars().any(char::is_control)
        {
            return Err("Package names must not begin with '-' or contain whitespace".into());
        }
    }
    let (binary, mut args) = match ecosystem {
        "cargo" => {
            let mut args = vec!["update".to_owned()];
            for name in package_names {
                args.push("-p".into());
                args.push(name.clone());
            }
            (config.cargo_binary.clone(), args)
        }
        "npm" => {
            let mut args = vec!["update".to_owned()];
            args.extend(package_names.iter().cloned());
            (config.npm_binary.clone(), args)
        }
        "bun" => {
            let mut args = vec!["update".to_owned()];
            args.extend(package_names.iter().cloned());
            (config.bun_binary.clone(), args)
        }
        _ => return Err(format!("Unsupported project package manager '{ecosystem}'")),
    };
    validate_value("package-manager executable", &binary)?;
    let display = render_command(&binary, &args);
    Ok(
        UninstallCommand::direct(binary, std::mem::take(&mut args), display, None)
            .in_directory(project_root.to_path_buf()),
    )
}

fn build_project_command(
    ecosystem: &str,
    root: &std::path::Path,
    package_name: &str,
    add: bool,
    config: &ProjectManagersConfig,
) -> Result<UninstallCommand, String> {
    validate_value("project package name", package_name)?;
    if package_name.starts_with('-')
        || package_name.chars().any(char::is_whitespace)
        || package_name.chars().any(char::is_control)
    {
        return Err("Package names must not begin with '-' or contain whitespace".to_owned());
    }
    let (binary, verb) = match ecosystem {
        "cargo" => (&config.cargo_binary, if add { "add" } else { "remove" }),
        "npm" => (
            &config.npm_binary,
            if add { "install" } else { "uninstall" },
        ),
        "bun" => (&config.bun_binary, if add { "add" } else { "remove" }),
        _ => return Err(format!("Unsupported project manager '{ecosystem}'")),
    };
    validate_value("package-manager executable", binary)?;
    let args = vec![verb.to_owned(), package_name.to_owned()];
    let display = render_command(binary, &args);
    Ok(
        UninstallCommand::direct(binary.clone(), args, display, None)
            .in_directory(root.to_path_buf()),
    )
}

fn parse_project_manager_id(value: &str) -> Option<(&str, &std::path::Path)> {
    let rest = value.strip_prefix("project:")?;
    let (ecosystem, root) = rest.split_once(':')?;
    Some((ecosystem, std::path::Path::new(root)))
}

/// Build one update command for a homogeneous group of packages already known
/// to have an available update.
pub fn build_update_command(
    packages: &[UnifiedPackage],
    behavior: &BehaviorConfig,
    projects: &ProjectManagersConfig,
) -> Result<UninstallCommand, String> {
    let first = packages.first().ok_or("No update targets selected")?;
    let names = packages
        .iter()
        .map(|package| {
            let name = validate_value("package name", &package.name)?;
            if name.starts_with('-')
                || name.chars().any(char::is_whitespace)
                || name.chars().any(char::is_control)
            {
                return Err(String::from(
                    "Package names must not begin with '-' or contain whitespace",
                ));
            }
            Ok(name.to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if packages
        .iter()
        .any(|package| package.source != first.source || package.manager_id != first.manager_id)
    {
        return Err("Update commands must contain packages from one manager instance".to_owned());
    }
    let (binary, mut args, cwd) = match first.source {
        PackageSource::Pacman => (
            "pacman".to_owned(),
            vec![
                // Arch does not support partial upgrades; a repository update must
                // use -Syu even when the user explicitly selected package targets.
                "-Syu".to_owned(),
                "--needed".to_owned(),
                "--noconfirm".to_owned(),
                "--".to_owned(),
            ],
            None,
        ),
        PackageSource::Aur => (
            "yay".to_owned(),
            vec![
                "-S".to_owned(),
                "--aur".to_owned(),
                "--needed".to_owned(),
                "--noconfirm".to_owned(),
                "--sudoflags".to_owned(),
                "-S".to_owned(),
                "--".to_owned(),
            ],
            None,
        ),
        PackageSource::Flatpak => (
            "flatpak".to_owned(),
            vec![
                "update".into(),
                "--noninteractive".into(),
                "--assumeyes".into(),
            ],
            None,
        ),
        PackageSource::Cargo => (
            "cargo".to_owned(),
            vec!["install".into(), "--force".into()],
            None,
        ),
        PackageSource::Uv => ("uv".to_owned(), vec!["tool".into(), "upgrade".into()], None),
        PackageSource::Pip => (
            "pip".to_owned(),
            vec!["install".into(), "--upgrade".into()],
            None,
        ),
        PackageSource::Project => {
            let manager_id = first
                .manager_id
                .as_deref()
                .ok_or("Project package is missing manager identity")?;
            let (ecosystem, root) =
                parse_project_manager_id(manager_id).ok_or("Invalid project manager identity")?;
            if !projects.supports(ecosystem) {
                return Err(format!(
                    "Project manager '{ecosystem}' is disabled in config.toml"
                ));
            }
            let (binary, args) = match ecosystem {
                // Cargo's `update -p` uses the package name, and is the actual
                // lockfile mutation corresponding to the dry-run discovery.
                "cargo" => (projects.cargo_binary.clone(), {
                    let mut args = vec!["update".to_owned()];
                    for name in &names {
                        args.push("-p".to_owned());
                        args.push(name.clone());
                    }
                    args
                }),
                // npm update accepts explicit package names and updates the
                // lockfile within declared semver ranges.
                "npm" => (projects.npm_binary.clone(), {
                    let mut args = vec!["update".to_owned()];
                    args.extend(names.clone());
                    args
                }),
                // Bun uses `bun update <package...>`.
                "bun" => (projects.bun_binary.clone(), {
                    let mut args = vec!["update".to_owned()];
                    args.extend(names.clone());
                    args
                }),
                _ => return Err(format!("Unsupported project manager '{ecosystem}'")),
            };
            (binary, args, Some(root.to_path_buf()))
        }
    };
    if first.source != PackageSource::Project {
        args.extend(names);
    }
    let display = render_command(&binary, &args);
    if first.source == PackageSource::Pacman {
        let (program, args, display, sudo_index) =
            privileged_command(&binary, args, behavior, true)?;
        Ok(UninstallCommand::direct(program, args, display, sudo_index))
    } else if first.source == PackageSource::Aur {
        Ok(UninstallCommand::direct(binary, args, display, None).with_stdin_password())
    } else {
        let command = UninstallCommand::direct(binary, args, display, None);
        Ok(match cwd {
            Some(directory) => command.in_directory(directory),
            None => command,
        })
    }
}

fn build_custom_command(
    manager: &CustomManagerConfig,
    name: &str,
    version: &str,
    behavior: &BehaviorConfig,
) -> Result<UninstallCommand, String> {
    if manager.uninstall_cmd.trim().is_empty() {
        return Err(format!(
            "Custom manager '{}' has no uninstall command",
            manager.id
        ));
    }
    let rendered = render_template(&manager.uninstall_cmd, name, version);
    let (program, args, display, sudo_index) = if manager.need_sudo {
        shell_command_with_sudo(&rendered, behavior)?
    } else {
        (
            "sh".to_owned(),
            vec!["-c".to_owned(), rendered.clone()],
            rendered,
            None,
        )
    };
    Ok(UninstallCommand::direct(program, args, display, sudo_index))
}

/// Expand placeholders in one pass so placeholder-like text inside a package
/// name or version is never interpreted as part of the template.
fn render_template(template: &str, name: &str, version: &str) -> String {
    let mut rendered = String::with_capacity(template.len() + name.len() + version.len());
    let mut rest = template;
    while let Some(index) = rest.find('{') {
        rendered.push_str(&rest[..index]);
        rest = &rest[index..];
        if let Some(value) = rest.strip_prefix("{name}") {
            rendered.push_str(&shell_quote(name));
            rest = value;
        } else if let Some(value) = rest.strip_prefix("{version}") {
            rendered.push_str(&shell_quote(version));
            rest = value;
        } else {
            rendered.push('{');
            rest = &rest[1..];
        }
    }
    rendered.push_str(rest);
    rendered
}

fn validate_value<'a>(label: &str, value: &'a str) -> Result<&'a str, String> {
    if value.is_empty() || value.contains('\0') {
        Err(format!("Invalid {label}"))
    } else {
        Ok(value)
    }
}

/// Escape a string as one POSIX shell word using single quotes.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn privileged_command(
    binary: &str,
    args: Vec<String>,
    behavior: &BehaviorConfig,
    needs_privileges: bool,
) -> Result<(String, Vec<String>, String, Option<usize>), String> {
    if needs_privileges {
        let (sudo, sudo_args) = sudo_parts(behavior)?;
        let mut full_args = sudo_args;
        let sudo_index = is_sudo_executable(&sudo).then_some(full_args.len());
        // Keep normal execution non-interactive. The TUI replaces this flag
        // with sudo's stdin-password mode only after the user confirms.
        full_args.push("-n".to_owned());
        full_args.push(binary.to_owned());
        let display = render_command(binary, &args);
        full_args.extend(args);
        Ok((sudo, full_args, display, sudo_index))
    } else {
        let display = render_command(binary, &args);
        Ok((binary.to_owned(), args, display, None))
    }
}

fn shell_command_with_sudo(
    script: &str,
    behavior: &BehaviorConfig,
) -> Result<(String, Vec<String>, String, Option<usize>), String> {
    let (sudo, mut args) = sudo_parts(behavior)?;
    let sudo_index = is_sudo_executable(&sudo).then_some(args.len());
    args.push("-n".to_owned());
    args.push("sh".to_owned());
    args.push("-c".to_owned());
    args.push(script.to_owned());
    let display = render_command(&sudo, &args);
    Ok((sudo, args, display, sudo_index))
}

fn is_sudo_executable(program: &str) -> bool {
    std::path::Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        == Some("sudo")
}

fn sudo_parts(behavior: &BehaviorConfig) -> Result<(String, Vec<String>), String> {
    let mut parts = behavior.sudo_command.split_whitespace();
    let binary = parts
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Configured sudo command is empty".to_owned())?;
    Ok((binary.to_owned(), parts.map(str::to_owned).collect()))
}

fn render_command(program: &str, args: &[String]) -> String {
    std::iter::once(program)
        .chain(args.iter().map(String::as_str))
        .map(shell_quote)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_templates_escape_package_values_and_respect_sudo() {
        let mut package =
            UnifiedPackage::new("odd name; touch /tmp/pwn", "1'2", PackageSource::Pip);
        package.manager_id = Some("npm".into());
        let mut manager = sample_manager();
        manager.uninstall_cmd = "npm uninstall {name} --version {version}".into();
        manager.need_sudo = true;
        let mut behavior = BehaviorConfig::default();
        behavior.sudo_command = "doas".into();

        let command = build_uninstall_command(&package, &behavior, &[manager]).unwrap();
        assert_eq!(command.program(), "doas");
        assert_eq!(command.args()[0], "-n");
        assert_eq!(command.args()[1], "sh");
        assert_eq!(
            command.args()[3],
            "npm uninstall 'odd name; touch /tmp/pwn' --version '1'\\''2'"
        );
    }

    #[test]
    fn custom_update_uses_configured_sudo_prompt_when_requested() {
        let behavior = BehaviorConfig::default();
        let command = UninstallCommand::custom_update(
            "pkgctl".into(),
            vec!["upgrade".into(), "demo".into()],
            &behavior,
            true,
        )
        .unwrap();

        assert_eq!(command.program(), "sudo");
        assert_eq!(command.args(), ["-n", "pkgctl", "upgrade", "demo"]);
        assert_eq!(command.display, "'pkgctl' 'upgrade' 'demo'");
        assert!(command.requires_stdin_password());
        assert_eq!(
            command
                .command_with_stdin_password()
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            ["-k", "-S", "-p", "", "pkgctl", "upgrade", "demo"]
        );
    }

    #[test]
    fn custom_update_without_privileges_runs_directly() {
        let behavior = BehaviorConfig::default();
        let command = UninstallCommand::custom_update(
            "pkgctl".into(),
            vec!["upgrade".into(), "demo".into()],
            &behavior,
            false,
        )
        .unwrap();

        assert_eq!(command.program(), "pkgctl");
        assert_eq!(command.args(), ["upgrade", "demo"]);
        assert!(!command.requires_stdin_password());
    }

    #[test]
    fn custom_update_with_doas_does_not_use_sudo_stdin_protocol() {
        let mut behavior = BehaviorConfig::default();
        behavior.sudo_command = "doas".into();
        let command = UninstallCommand::custom_update(
            "pkgctl".into(),
            vec!["upgrade".into(), "demo".into()],
            &behavior,
            true,
        )
        .unwrap();

        assert_eq!(command.program(), "doas");
        assert_eq!(command.args(), ["-n", "pkgctl", "upgrade", "demo"]);
        assert!(!command.requires_stdin_password());
    }

    #[test]
    fn pacman_uses_option_terminator_and_sudo() {
        let package = UnifiedPackage::new("-name", "1", PackageSource::Pacman);
        let command = build_uninstall_command(&package, &BehaviorConfig::default(), &[]).unwrap();
        assert_eq!(command.program(), "sudo");
        assert_eq!(
            command.args(),
            ["-n", "pacman", "-R", "--noconfirm", "--", "-name"]
        );
        assert_eq!(command.sudo_program(), Some("sudo"));
    }

    #[test]
    fn sudo_password_mode_reads_stdin_and_hides_sudo_from_preview() {
        let package = UnifiedPackage::new("app", "1", PackageSource::Pacman);
        let command = build_uninstall_command(&package, &BehaviorConfig::default(), &[]).unwrap();

        assert_eq!(command.display, "'pacman' '-R' '--noconfirm' '--' 'app'");
        let prepared = command.command_with_stdin_password();
        let args = prepared
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            args,
            [
                "-k",
                "-S",
                "-p",
                "",
                "pacman",
                "-R",
                "--noconfirm",
                "--",
                "app"
            ]
        );
        assert!(!args.iter().any(|arg| arg == "-n"));
    }

    #[test]
    fn doas_operations_do_not_request_a_sudo_prompt() {
        let package = UnifiedPackage::new("app", "1", PackageSource::Pacman);
        let mut behavior = BehaviorConfig::default();
        behavior.sudo_command = "doas".into();
        let command = build_uninstall_command(&package, &behavior, &[]).unwrap();
        assert_eq!(command.sudo_program(), None);
        assert!(!command.requires_stdin_password());
    }

    #[test]
    fn pacman_removal_does_not_cascade_into_dependencies() {
        let package = UnifiedPackage::new("app", "1", PackageSource::Pacman);
        let command = build_uninstall_command(&package, &BehaviorConfig::default(), &[]).unwrap();
        assert!(command.args().contains(&"-R".to_owned()));
        assert!(!command.args().iter().any(|arg| arg == "-Rns"));
    }

    #[test]
    fn custom_template_does_not_expand_placeholders_inside_values() {
        let mut package = UnifiedPackage::new("pkg{version}; echo unsafe", "2", PackageSource::Pip);
        package.manager_id = Some("npm".into());
        let mut manager = sample_manager();
        manager.uninstall_cmd = "npm remove {name} --version {version}".into();

        let command =
            build_uninstall_command(&package, &BehaviorConfig::default(), &[manager]).unwrap();
        assert_eq!(
            command.args()[1],
            "npm remove 'pkg{version}; echo unsafe' --version '2'"
        );
    }

    fn sample_manager() -> CustomManagerConfig {
        CustomManagerConfig {
            id: "npm".into(),
            name: "npm".into(),
            badge: "[NPM]".into(),
            color: None,
            binary: "npm".into(),
            list_cmd: "npm list".into(),
            format: crate::config::OutputFormat::Json,
            parse_regex: None,
            json_array_path: None,
            json_map_path: None,
            json_name: None,
            json_version: None,
            json_size: None,
            json_desc: None,
            size_cmd: None,
            info_cmd: None,
            uninstall_cmd: "npm uninstall {name}".into(),
            need_sudo: false,
            update_check_cmd: None,
            update_format: None,
            update_parse_regex: None,
            update_json_array_path: None,
            update_json_name: None,
            update_json_latest_version: None,
            update_cmd: None,
        }
    }
}
