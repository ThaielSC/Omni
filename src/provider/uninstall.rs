//! Safe command construction for uninstalling installed packages.
//!
//! Native providers are invoked directly with argument vectors. Custom manager
//! commands are explicitly shell-based because their TOML command templates may
//! contain normal shell syntax; package fields are POSIX-shell escaped before
//! being substituted into those templates.

use crate::config::{BehaviorConfig, CustomManagerConfig};
use crate::model::{PackageSource, UnifiedPackage};
use std::process::Command;

/// A fully rendered uninstall operation, suitable for confirmation UI and execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UninstallCommand {
    /// Human-readable command preview shown before execution.
    pub display: String,
    program: String,
    args: Vec<String>,
}

impl UninstallCommand {
    fn direct(program: impl Into<String>, args: Vec<String>, display: String) -> Self {
        Self {
            display,
            program: program.into(),
            args,
        }
    }

    /// Build a fresh process command for this operation.
    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
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
    /// The TUI may temporarily leave raw mode and run `sudo -v` on the user's
    /// terminal before executing this command non-interactively.
    pub fn sudo_program(&self) -> Option<&str> {
        (std::path::Path::new(&self.program).file_name()?.to_str()? == "sudo")
            .then_some(self.program.as_str())
    }
}

/// Construct an uninstall command for a package from a supported native or
/// user-defined manager. This function only prepares a command; it never runs it.
pub fn build_uninstall_command(
    package: &UnifiedPackage,
    behavior: &BehaviorConfig,
    managers: &[CustomManagerConfig],
) -> Result<UninstallCommand, String> {
    let name = validate_value("package name", &package.name)?;
    let version = validate_value("package version", &package.version)?;

    if let Some(manager_id) = package.manager_id.as_deref() {
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
            let (program, args, display) = privileged_command("pacman", args, behavior, true)?;
            Ok(UninstallCommand::direct(program, args, display))
        }
        PackageSource::Flatpak => {
            let args = vec![
                "uninstall".to_owned(),
                "--noninteractive".to_owned(),
                "--".to_owned(),
                name.to_owned(),
            ];
            let (program, args, display) = privileged_command("flatpak", args, behavior, false)?;
            Ok(UninstallCommand::direct(program, args, display))
        }
        PackageSource::Cargo => {
            let args = vec!["uninstall".to_owned(), name.to_owned()];
            let (program, args, display) = privileged_command("cargo", args, behavior, false)?;
            Ok(UninstallCommand::direct(program, args, display))
        }
        PackageSource::Uv => {
            let args = vec!["tool".to_owned(), "uninstall".to_owned(), name.to_owned()];
            let (program, args, display) = privileged_command("uv", args, behavior, false)?;
            Ok(UninstallCommand::direct(program, args, display))
        }
        PackageSource::Pip => {
            let args = vec!["uninstall".to_owned(), "-y".to_owned(), name.to_owned()];
            let (program, args, display) = privileged_command("pip", args, behavior, false)?;
            Ok(UninstallCommand::direct(program, args, display))
        }
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
    let (program, args, display) = if manager.need_sudo {
        shell_command_with_sudo(&rendered, behavior)?
    } else {
        (
            "sh".to_owned(),
            vec!["-c".to_owned(), rendered.clone()],
            rendered,
        )
    };
    Ok(UninstallCommand::direct(program, args, display))
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
) -> Result<(String, Vec<String>, String), String> {
    if needs_privileges {
        let (sudo, sudo_args) = sudo_parts(behavior)?;
        let mut full_args = sudo_args;
        // The TUI obtains sudo authorization in a separate terminal prompt
        // before starting this non-interactive subprocess.
        full_args.push("-n".to_owned());
        full_args.push(binary.to_owned());
        full_args.extend(args);
        let display = render_command(&sudo, &full_args);
        Ok((sudo, full_args, display))
    } else {
        let display = render_command(binary, &args);
        Ok((binary.to_owned(), args, display))
    }
}

fn shell_command_with_sudo(
    script: &str,
    behavior: &BehaviorConfig,
) -> Result<(String, Vec<String>, String), String> {
    let (sudo, mut args) = sudo_parts(behavior)?;
    args.push("-n".to_owned());
    args.push("sh".to_owned());
    args.push("-c".to_owned());
    args.push(script.to_owned());
    let display = render_command(&sudo, &args);
    Ok((sudo, args, display))
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
    fn doas_operations_do_not_request_a_sudo_prompt() {
        let package = UnifiedPackage::new("app", "1", PackageSource::Pacman);
        let mut behavior = BehaviorConfig::default();
        behavior.sudo_command = "doas".into();
        let command = build_uninstall_command(&package, &behavior, &[]).unwrap();
        assert_eq!(command.sudo_program(), None);
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
        }
    }
}
