//! Read-only update discovery for native package managers.
//!
//! This module only queries package managers. It never invokes install, upgrade,
//! or mutation commands. UI code can consume the per-manager status even when a
//! manager is absent or its optional update helper is unsupported.

use crate::model::{PackageSource, UnifiedPackage};
use serde_json::Value;
use std::collections::HashMap;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateManager {
    Flatpak,
    Uv,
    Pip,
    Cargo,
}

impl UpdateManager {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Flatpak => "flatpak",
            Self::Uv => "uv",
            Self::Pip => "pip",
            Self::Cargo => "cargo",
        }
    }

    pub fn source(self) -> PackageSource {
        match self {
            Self::Flatpak => PackageSource::Flatpak,
            Self::Uv => PackageSource::Uv,
            Self::Pip => PackageSource::Pip,
            Self::Cargo => PackageSource::Cargo,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableUpdate {
    pub manager: UpdateManager,
    pub package_name: String,
    pub installed_version: Option<String>,
    pub latest_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheckState {
    Checked,
    Unavailable,
    Unsupported,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheckReport {
    pub manager: UpdateManager,
    pub state: UpdateCheckState,
    pub updates: Vec<AvailableUpdate>,
}

impl UpdateCheckReport {
    fn new(manager: UpdateManager, state: UpdateCheckState, updates: Vec<AvailableUpdate>) -> Self {
        Self {
            manager,
            state,
            updates,
        }
    }
}

/// Run read-only update queries for Flatpak, uv, pip, and Cargo global tools.
///
/// `installed` is used to attach the currently installed version and to avoid
/// presenting remote candidates that are not present in Omni's inventory.
/// Each manager reports independently, so one unavailable manager does not
/// suppress results from the others.
pub fn discover_updates(installed: &[UnifiedPackage]) -> Vec<UpdateCheckReport> {
    vec![
        check_flatpak(installed),
        check_uv(installed),
        check_pip(installed),
        check_cargo(installed),
    ]
}

fn check_flatpak(installed: &[UnifiedPackage]) -> UpdateCheckReport {
    let manager = UpdateManager::Flatpak;
    check_command(
        manager,
        installed,
        "flatpak",
        &[
            "remote-ls",
            "--updates",
            "--app",
            "--columns=application,version",
        ],
        |out| parse_flatpak_updates(out, installed),
    )
}

fn check_uv(installed: &[UnifiedPackage]) -> UpdateCheckReport {
    let manager = UpdateManager::Uv;
    check_command(
        manager,
        installed,
        "uv",
        &["tool", "list", "--outdated"],
        |out| parse_uv_updates(out, installed),
    )
}

fn check_pip(installed: &[UnifiedPackage]) -> UpdateCheckReport {
    let manager = UpdateManager::Pip;
    check_command(
        manager,
        installed,
        "pip",
        &["list", "--outdated", "--format=json"],
        |out| parse_pip_updates(out, installed),
    )
}

fn check_cargo(installed: &[UnifiedPackage]) -> UpdateCheckReport {
    let manager = UpdateManager::Cargo;
    if !installed
        .iter()
        .any(|package| package.source == manager.source())
    {
        return UpdateCheckReport::new(manager, UpdateCheckState::Checked, Vec::new());
    }
    if !super::available("cargo") {
        return UpdateCheckReport::new(manager, UpdateCheckState::Unavailable, Vec::new());
    }

    // `install-update` is supplied by the optional cargo-update component.
    // Exit failure generally means that helper is not installed; do not treat
    // that as a failed Cargo inventory scan or suggest that updates were found.
    let output = match Command::new("cargo")
        .args(["install-update", "--list"])
        .output()
    {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            let missing_helper = stderr.contains("no such command")
                || stderr.contains("unrecognized subcommand")
                || stderr.contains("unknown command");
            let state = if missing_helper {
                UpdateCheckState::Unsupported
            } else {
                UpdateCheckState::Failed(if stderr.is_empty() {
                    format!("cargo install-update exited with status {}", output.status)
                } else {
                    stderr
                })
            };
            return UpdateCheckReport::new(manager, state, Vec::new());
        }
        Err(error) => {
            return UpdateCheckReport::new(
                manager,
                UpdateCheckState::Failed(error.to_string()),
                Vec::new(),
            );
        }
    };
    let output = match String::from_utf8(output.stdout) {
        Ok(output) => output,
        Err(error) => {
            return UpdateCheckReport::new(
                manager,
                UpdateCheckState::Failed(format!(
                    "cargo install-update output is not UTF-8: {error}"
                )),
                Vec::new(),
            );
        }
    };
    match parse_cargo_updates(&output, installed) {
        Some(updates) => UpdateCheckReport::new(manager, UpdateCheckState::Checked, updates),
        None => UpdateCheckReport::new(manager, UpdateCheckState::Unsupported, Vec::new()),
    }
}

fn check_command(
    manager: UpdateManager,
    installed: &[UnifiedPackage],
    binary: &str,
    args: &[&str],
    parse: impl FnOnce(&str) -> Result<Vec<AvailableUpdate>, String>,
) -> UpdateCheckReport {
    if !installed
        .iter()
        .any(|package| package.source == manager.source())
    {
        return UpdateCheckReport::new(manager, UpdateCheckState::Checked, Vec::new());
    }
    if !super::available(binary) {
        return UpdateCheckReport::new(manager, UpdateCheckState::Unavailable, Vec::new());
    }
    let output = match Command::new(binary).args(args).output() {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            let message = if stderr.is_empty() {
                format!("{binary} exited with status {}", output.status)
            } else {
                stderr
            };
            return UpdateCheckReport::new(manager, UpdateCheckState::Failed(message), Vec::new());
        }
        Err(error) => {
            let state = if error.kind() == std::io::ErrorKind::NotFound {
                UpdateCheckState::Unavailable
            } else {
                UpdateCheckState::Failed(error.to_string())
            };
            return UpdateCheckReport::new(manager, state, Vec::new());
        }
    };
    match String::from_utf8(output.stdout)
        .map_err(|error| error.to_string())
        .and_then(|text| parse(&text))
    {
        Ok(updates) => UpdateCheckReport::new(manager, UpdateCheckState::Checked, updates),
        Err(error) => UpdateCheckReport::new(manager, UpdateCheckState::Failed(error), Vec::new()),
    }
}

/// Parse Flatpak's tab-separated `application,version` output.
/// Rows are matched against installed application IDs, and the version is
/// recorded as the candidate version returned by the remote.
pub fn parse_flatpak_updates(
    input: &str,
    installed: &[UnifiedPackage],
) -> Result<Vec<AvailableUpdate>, String> {
    let installed = installed_index(installed, PackageSource::Flatpak);
    let mut updates = Vec::new();
    for (line_number, line) in input.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 2 || fields[0].trim().is_empty() || fields[1].trim().is_empty() {
            return Err(format!("invalid Flatpak update row {}", line_number + 1));
        }
        let name = fields[0].trim();
        let latest = fields[1].trim();
        if let Some(current) = installed.get(&canonical_name(name)) {
            updates.push(AvailableUpdate {
                manager: UpdateManager::Flatpak,
                package_name: current.name.clone(),
                installed_version: Some(current.version.clone()),
                latest_version: latest.to_owned(),
            });
        }
    }
    Ok(updates)
}

/// Parse `uv tool list --outdated`, whose text rows include
/// `name v<installed> (latest: v<latest>)`.
pub fn parse_uv_updates(
    input: &str,
    installed: &[UnifiedPackage],
) -> Result<Vec<AvailableUpdate>, String> {
    let installed = installed_index(installed, PackageSource::Uv);
    let mut updates = Vec::new();
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("-") || line.starts_with("Installed ") {
            continue;
        }
        let Some((name, rest)) = line.split_once(char::is_whitespace) else {
            // Ignore informational lines, but require at least one recognized
            // row whenever there is non-empty output.
            continue;
        };
        let Some(latest) = parse_uv_latest(rest) else {
            continue;
        };
        let installed_version = parse_uv_installed(rest);
        if let Some(current) = installed.get(&canonical_name(name)) {
            updates.push(AvailableUpdate {
                manager: UpdateManager::Uv,
                package_name: current.name.clone(),
                installed_version: installed_version.or_else(|| Some(current.version.clone())),
                latest_version: latest,
            });
        }
    }
    if !input.trim().is_empty()
        && updates.is_empty()
        && !input.lines().all(|line| line.trim().is_empty())
    {
        // uv only emits outdated tools; non-empty output with no recognized
        // version row may reflect a future, changed output format.
        let recognized_any = input.lines().any(|line| parse_uv_latest(line).is_some());
        if !recognized_any {
            return Err("unrecognized `uv tool list --outdated` output".into());
        }
    }
    Ok(updates)
}

fn parse_uv_latest(rest: &str) -> Option<String> {
    let marker = "(latest:";
    let start = rest.find(marker)? + marker.len();
    let version = rest[start..].trim_start().trim_start_matches('v');
    let end = version.find(')')?;
    let version = version[..end].trim();
    (!version.is_empty()).then(|| version.to_owned())
}

fn parse_uv_installed(rest: &str) -> Option<String> {
    let version = rest.trim().strip_prefix('v')?;
    let end = version.find(char::is_whitespace).unwrap_or(version.len());
    let version = version[..end].trim();
    (!version.is_empty()).then(|| version.to_owned())
}

/// Parse `pip list --outdated --format=json` rows.
pub fn parse_pip_updates(
    input: &str,
    installed: &[UnifiedPackage],
) -> Result<Vec<AvailableUpdate>, String> {
    let root: Value = serde_json::from_str(input).map_err(|error| error.to_string())?;
    let rows = root
        .as_array()
        .ok_or_else(|| "pip outdated JSON must be an array".to_owned())?;
    let installed = installed_index(installed, PackageSource::Pip);
    let mut updates = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let name = row
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| format!("pip outdated row {} has no package name", index + 1))?;
        let latest = row
            .get("latest_version")
            .and_then(Value::as_str)
            .filter(|version| !version.trim().is_empty())
            .ok_or_else(|| format!("pip outdated row {} has no latest_version", index + 1))?;
        let reported_current = row.get("version").and_then(Value::as_str);
        if let Some(current) = installed.get(&canonical_name(name)) {
            updates.push(AvailableUpdate {
                manager: UpdateManager::Pip,
                package_name: current.name.clone(),
                installed_version: reported_current
                    .map(str::to_owned)
                    .or_else(|| Some(current.version.clone())),
                latest_version: latest.to_owned(),
            });
        }
    }
    Ok(updates)
}

/// Parse the stable table shape emitted by cargo-update's
/// `cargo install-update --list`. `None` means the command output is not in the
/// recognized format; callers should surface this as unsupported, not as an
/// empty successful scan.
pub fn parse_cargo_updates(
    input: &str,
    installed: &[UnifiedPackage],
) -> Option<Vec<AvailableUpdate>> {
    let installed = installed_index(installed, PackageSource::Cargo);
    let mut header_seen = false;
    let mut updates = Vec::new();
    for line in input.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() >= 4
            && fields[0].eq_ignore_ascii_case("package")
            && fields[1].eq_ignore_ascii_case("installed")
            && fields[2].eq_ignore_ascii_case("latest")
        {
            header_seen = true;
            continue;
        }
        if !header_seen || fields.len() < 4 {
            continue;
        }
        let name = fields[0];
        let installed_version = fields[1].trim_start_matches('v');
        let latest_version = fields[2].trim_start_matches('v');
        let needs_update = fields[3].eq_ignore_ascii_case("yes")
            || fields[3].eq_ignore_ascii_case("true")
            || fields[3] == "*";
        if !needs_update || latest_version == "-" || latest_version == installed_version {
            continue;
        }
        if let Some(current) = installed.get(&canonical_name(name)) {
            updates.push(AvailableUpdate {
                manager: UpdateManager::Cargo,
                package_name: current.name.clone(),
                installed_version: Some(installed_version.to_owned()),
                latest_version: latest_version.to_owned(),
            });
        }
    }
    header_seen.then_some(updates)
}

fn installed_index(
    packages: &[UnifiedPackage],
    source: PackageSource,
) -> HashMap<String, &UnifiedPackage> {
    packages
        .iter()
        .filter(|package| package.source == source)
        .map(|package| (canonical_name(&package.name), package))
        .collect()
}

fn canonical_name(name: &str) -> String {
    name.chars()
        .filter(|character| !matches!(character, '-' | '_' | '.'))
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(name: &str, version: &str, source: PackageSource) -> UnifiedPackage {
        UnifiedPackage::new(name, version, source)
    }

    #[test]
    fn parses_flatpak_update_rows_and_matches_installed_apps() {
        let installed = [
            package("org.example.Editor", "1.2", PackageSource::Flatpak),
            package("org.example.Other", "4", PackageSource::Flatpak),
            package("ripgrep", "14", PackageSource::Cargo),
        ];
        let updates = parse_flatpak_updates(
            "org.example.Editor\t1.3\norg.example.NotInstalled\t9\n",
            &installed,
        )
        .unwrap();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].package_name, "org.example.Editor");
        assert_eq!(updates[0].installed_version.as_deref(), Some("1.2"));
        assert_eq!(updates[0].latest_version, "1.3");
    }

    #[test]
    fn rejects_malformed_flatpak_rows() {
        assert!(parse_flatpak_updates("org.example.App\n", &[]).is_err());
    }

    #[test]
    fn parses_uv_outdated_tool_text() {
        let installed = [package("ruff", "0.11.0", PackageSource::Uv)];
        let updates = parse_uv_updates(
            "ruff v0.11.0 (latest: v0.11.2)\nblack v24.10.0 (latest: v25.1.0)\n",
            &installed,
        )
        .unwrap();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].installed_version.as_deref(), Some("0.11.0"));
        assert_eq!(updates[0].latest_version, "0.11.2");
    }

    #[test]
    fn accepts_empty_uv_result_but_rejects_unrecognized_nonempty_text() {
        assert!(parse_uv_updates("", &[]).unwrap().is_empty());
        assert!(parse_uv_updates("some future output\n", &[]).is_err());
    }

    #[test]
    fn parses_pip_outdated_json_and_pep_name_variants() {
        let installed = [package("charset_normalizer", "3.4.0", PackageSource::Pip)];
        let updates = parse_pip_updates(
            r#"[{"name":"charset-normalizer","version":"3.4.0","latest_version":"3.4.2","latest_filetype":"wheel"}]"#,
            &installed,
        )
        .unwrap();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].package_name, "charset_normalizer");
        assert_eq!(updates[0].latest_version, "3.4.2");
    }

    #[test]
    fn rejects_pip_json_without_expected_schema() {
        assert!(parse_pip_updates("{}", &[]).is_err());
        assert!(parse_pip_updates(r#"[{"name":"pkg","version":"1"}]"#, &[]).is_err());
    }

    #[test]
    fn cargo_helper_output_must_have_recognized_header_and_update_marker() {
        let installed = [package("ripgrep", "14.0", PackageSource::Cargo)];
        assert!(parse_cargo_updates("no helper output", &installed).is_none());
        let updates = parse_cargo_updates(
            "Package          Installed  Latest  Needs update\nripgrep          v14.0      v14.1   Yes\nother            1.0        2.0     No\n",
            &installed,
        )
        .unwrap();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].latest_version, "14.1");
    }

    #[test]
    fn canonicalizes_python_package_names_without_crossing_sources() {
        assert_eq!(canonical_name("A.B_c-D"), "abcd");
        let installed = [package("same", "1", PackageSource::Uv)];
        assert!(parse_pip_updates("[]", &installed).unwrap().is_empty());
    }

    #[test]
    fn skips_native_update_queries_when_inventory_has_no_matching_packages() {
        let reports = discover_updates(&[]);
        assert_eq!(reports.len(), 4);
        assert!(reports.iter().all(|report| {
            report.state == UpdateCheckState::Checked && report.updates.is_empty()
        }));
    }
}
