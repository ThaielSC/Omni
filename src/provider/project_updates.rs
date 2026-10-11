//! Read-only update discovery for dependencies declared by local projects.
//!
//! Results are keyed by both manager identity and dependency name because a
//! workspace can contain the same dependency name in multiple manifests.

use crate::config::ProjectManagersConfig;
use crate::model::{PackageSource, UnifiedPackage};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::process::{Command, Output};

/// Identifies a project dependency without conflating separate projects or
/// workspace members that happen to use the same package name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectPackageKey {
    pub manager_id: String,
    pub name: String,
}

/// Available compatible updates and one diagnostic per manager whose check
/// could not be completed. An absent result means either current or unchecked;
/// callers should distinguish those states using the returned errors/status.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectUpdateResults {
    pub updates: HashMap<ProjectPackageKey, String>,
    pub errors: HashMap<String, String>,
}

#[derive(Debug)]
struct ManagerPackages {
    ecosystem: String,
    root: PathBuf,
    names: HashSet<String>,
}

/// Query project-local managers for newer versions while keeping manifests and
/// lockfiles untouched. Cargo uses a whole-lockfile `--dry-run` and retains only
/// direct dependencies in the result; npm's `outdated` and Bun's
/// `outdated` commands are documented read-only queries.
pub fn check_project_updates(
    packages: &[UnifiedPackage],
    config: &ProjectManagersConfig,
) -> ProjectUpdateResults {
    let mut managers: BTreeMap<String, ManagerPackages> = BTreeMap::new();
    for package in packages {
        if package.source != PackageSource::Project {
            continue;
        }
        let Some(manager_id) = package.manager_id.as_deref() else {
            continue;
        };
        let Some((ecosystem, root)) = parse_project_manager_id(manager_id) else {
            continue;
        };
        if !config.supports(&ecosystem) {
            continue;
        }
        managers
            .entry(manager_id.to_owned())
            .or_insert_with(|| ManagerPackages {
                ecosystem,
                root,
                names: HashSet::new(),
            })
            .names
            .insert(package.name.clone());
    }

    let mut results = ProjectUpdateResults::default();
    for (manager_id, manager) in managers {
        let query = match manager.ecosystem.as_str() {
            "cargo" => query_cargo(&manager, &config.cargo_binary),
            "npm" => query_npm(&manager, &config.npm_binary),
            "bun" => query_bun(&manager, &config.bun_binary),
            other => Err(format!("Unsupported project package manager '{other}'")),
        };
        match query {
            Ok(updates) => {
                results
                    .updates
                    .extend(updates.into_iter().map(|(name, version)| {
                        (
                            ProjectPackageKey {
                                manager_id: manager_id.clone(),
                                name,
                            },
                            version,
                        )
                    }));
            }
            Err(error) => {
                results.errors.insert(manager_id, error);
            }
        }
    }
    results
}

fn parse_project_manager_id(manager_id: &str) -> Option<(String, PathBuf)> {
    let remainder = manager_id.strip_prefix("project:")?;
    let (ecosystem, root) = remainder.split_once(':')?;
    if root.is_empty() {
        return None;
    }
    Some((ecosystem.to_owned(), PathBuf::from(root)))
}

fn query_cargo(manager: &ManagerPackages, binary: &str) -> Result<HashMap<String, String>, String> {
    if manager.names.is_empty() {
        return Ok(HashMap::new());
    }
    let mut command = Command::new(binary);
    command
        .current_dir(&manager.root)
        .arg("update")
        .arg("--dry-run")
        .arg("--manifest-path")
        .arg(manager.root.join("Cargo.toml"));
    command.env("NO_COLOR", "1");
    let output = command
        .output()
        .map_err(|error| format!("could not start {binary} update --dry-run: {error}"))?;
    ensure_success("cargo update --dry-run", &output)?;
    Ok(parse_cargo_dry_run(
        &combined_output(&output),
        &manager.names,
    ))
}

fn query_npm(manager: &ManagerPackages, binary: &str) -> Result<HashMap<String, String>, String> {
    let output = Command::new(binary)
        .current_dir(&manager.root)
        .args(["outdated", "--json"])
        .env("NO_COLOR", "1")
        .output()
        .map_err(|error| format!("could not start {binary} outdated --json: {error}"))?;

    // npm exits with status 1 when outdated packages are found. The JSON
    // payload is still the useful result in that case; other failures report
    // diagnostics through stderr and are surfaced to the caller.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed = serde_json::from_str::<serde_json::Value>(&stdout);
    if !output.status.success() && output.status.code() != Some(1) {
        return Err(command_error(&format!("{binary} outdated --json"), &output));
    }
    let value = parsed.map_err(|error| {
        if !output.status.success() && output.status.code() == Some(1) {
            format!(
                "npm outdated returned status 1 but no valid JSON result: {}; {error}",
                diagnostic(&output)
            )
        } else {
            format!("npm outdated returned invalid JSON: {error}")
        }
    })?;
    parse_npm_outdated(&value, &manager.names)
}

fn parse_npm_outdated(
    value: &serde_json::Value,
    direct_names: &HashSet<String>,
) -> Result<HashMap<String, String>, String> {
    let Some(entries) = value.as_object() else {
        return Err("npm outdated returned JSON that was not an object".to_owned());
    };

    let mut updates = HashMap::new();
    for name in direct_names {
        let Some(entry) = entries.get(name).and_then(serde_json::Value::as_object) else {
            continue;
        };
        let current = entry.get("current").and_then(serde_json::Value::as_str);
        let wanted = entry.get("wanted").and_then(serde_json::Value::as_str);
        if let (Some(current), Some(wanted)) = (current, wanted)
            && !is_missing_version(current)
            && !is_missing_version(wanted)
            && !same_version(current, wanted)
        {
            // `wanted` is the newest version allowed by package.json's range;
            // the `latest` field can be a breaking version outside that range.
            updates.insert(name.clone(), wanted.to_owned());
        }
    }
    Ok(updates)
}

fn query_bun(manager: &ManagerPackages, binary: &str) -> Result<HashMap<String, String>, String> {
    let output = Command::new(binary)
        .current_dir(&manager.root)
        .args(["outdated", "--no-progress"])
        .env("NO_COLOR", "1")
        .output()
        .map_err(|error| format!("could not start {binary} outdated: {error}"))?;
    // Bun documents `bun outdated` as read-only. Accept status 1 for the
    // common CLI convention where outdated dependencies are represented as a
    // non-zero result, but still require a parseable table.
    if !output.status.success() && output.status.code() != Some(1) {
        return Err(command_error(&format!("{binary} outdated"), &output));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let updates = parse_bun_outdated(&stdout, &manager.names);
    if !output.status.success() && updates.is_empty() {
        return Err(format!(
            "{binary} outdated returned status 1 without parseable update rows: {}",
            diagnostic(&output)
        ));
    }
    Ok(updates)
}

fn ensure_success(label: &str, output: &Output) -> Result<(), String> {
    if output.status.success() {
        Ok(())
    } else {
        Err(command_error(label, output))
    }
}

fn command_error(label: &str, output: &Output) -> String {
    format!("{label} failed ({}): {}", output.status, diagnostic(output))
}

fn diagnostic(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let message = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    if message.is_empty() {
        "no diagnostic output".to_owned()
    } else {
        message.chars().take(500).collect()
    }
}

fn combined_output(output: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn parse_cargo_dry_run(output: &str, direct_names: &HashSet<String>) -> HashMap<String, String> {
    let mut updates = HashMap::new();
    for line in output.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 5 || fields[0] != "Updating" {
            continue;
        }
        let name = fields[1];
        if !direct_names.contains(name) || fields[3] != "->" {
            continue;
        }
        if let Some(version) = fields[4].strip_prefix('v')
            && !version.is_empty()
        {
            updates.insert(name.to_owned(), version.to_owned());
        }
    }
    updates
}

fn parse_bun_outdated(output: &str, direct_names: &HashSet<String>) -> HashMap<String, String> {
    let mut columns = None;
    let mut updates = HashMap::new();
    for raw_line in output.lines() {
        let line = raw_line.trim();
        if !line.contains('|') {
            continue;
        }
        let cells = line
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect::<Vec<_>>();
        if cells
            .iter()
            .any(|cell| cell.eq_ignore_ascii_case("package"))
            && cells.iter().any(|cell| cell.eq_ignore_ascii_case("update"))
        {
            let package_index = cells
                .iter()
                .position(|cell| cell.eq_ignore_ascii_case("package"));
            let current_index = cells
                .iter()
                .position(|cell| cell.eq_ignore_ascii_case("current"));
            let update_index = cells
                .iter()
                .position(|cell| cell.eq_ignore_ascii_case("update"));
            if let (Some(package), Some(current), Some(update)) =
                (package_index, current_index, update_index)
            {
                columns = Some((package, current, update));
            }
            continue;
        }
        let Some((package_index, current_index, update_index)) = columns else {
            continue;
        };
        let max_index = package_index.max(current_index).max(update_index);
        if cells.len() <= max_index {
            continue;
        }
        // Bun annotates rows with their declaration type, e.g. `react (dev)`.
        let display_name = cells[package_index];
        let name = display_name
            .strip_suffix(')')
            .and_then(|value| value.rsplit_once(" (").map(|(name, _)| name))
            .unwrap_or(display_name);
        let current = cells[current_index];
        let update = cells[update_index];
        if direct_names.contains(name)
            && !is_missing_version(current)
            && !is_missing_version(update)
            && !same_version(current, update)
        {
            updates.insert(name.to_owned(), update.to_owned());
        }
    }
    updates
}

fn is_missing_version(version: &str) -> bool {
    matches!(version.trim(), "" | "-" | "(empty)" | "MISSING")
}

fn same_version(left: &str, right: &str) -> bool {
    left.trim().trim_start_matches('v') == right.trim().trim_start_matches('v')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProjectManagersConfig;
    use crate::model::{PackageSource, UnifiedPackage};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn names(values: &[&str]) -> HashSet<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn cargo_parser_returns_only_requested_direct_crates() {
        let output = "Updating crates.io index\nUpdating serde v1.0.1 -> v1.0.2\nUpdating transitive v2.0.0 -> v3.0.0\n";
        let updates = parse_cargo_dry_run(output, &names(&["serde"]));
        assert_eq!(updates.get("serde").map(String::as_str), Some("1.0.2"));
        assert_eq!(updates.len(), 1);
    }

    #[test]
    fn cargo_update_query_avoids_ambiguous_package_name_filters() {
        let temp = std::env::temp_dir().join(format!(
            "omni-cargo-update-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp).unwrap();
        let project = temp.join("project");
        fs::create_dir_all(&project).unwrap();
        let binary = temp.join("cargo-test");
        fs::write(
            &binary,
            "#!/bin/sh\nfor arg in \"$@\"; do [ \"$arg\" != \"-p\" ] || exit 42; done\nprintf '%s\\n' 'Updating thiserror v2.0.0 -> v2.0.1' 'Updating transitive v1.0.0 -> v1.0.1'\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&binary).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&binary, permissions).unwrap();
        let manager = ManagerPackages {
            ecosystem: "cargo".into(),
            root: project,
            names: names(&["thiserror"]),
        };

        let updates = query_cargo(&manager, binary.to_str().unwrap()).unwrap();
        assert_eq!(updates.get("thiserror").map(String::as_str), Some("2.0.1"));
        assert_eq!(updates.len(), 1);
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn npm_parser_uses_semver_compatible_wanted_version() {
        let output = r#"{"react":{"current":"18.2.0","wanted":"18.3.0","latest":"19.0.0"},"same":{"current":"1.0.0","wanted":"1.0.0","latest":"2.0.0"}}"#;
        let value: serde_json::Value = serde_json::from_str(output).unwrap();
        let updates = parse_npm_outdated(&value, &names(&["react", "same"])).unwrap();
        assert_eq!(updates.get("react").map(String::as_str), Some("18.3.0"));
        assert!(!updates.contains_key("same"));
    }

    #[test]
    fn bun_parser_handles_table_and_ignores_transitive_names() {
        let output = "| Package | Current | Update | Latest |\n| --- | --- | --- | --- |\n| react (dev) | 18.2.0 | 18.3.0 | 19.0.0 |\n| transitive | 1.0.0 | 2.0.0 | 2.0.0 |\n";
        let updates = parse_bun_outdated(output, &names(&["react"]));
        assert_eq!(updates.get("react").map(String::as_str), Some("18.3.0"));
        assert_eq!(updates.len(), 1);
    }

    #[test]
    fn manager_id_keeps_project_root() {
        let (ecosystem, root) =
            parse_project_manager_id("project:npm:/tmp/a:b").expect("valid manager id");
        assert_eq!(ecosystem, "npm");
        assert_eq!(root, PathBuf::from("/tmp/a:b"));
    }

    #[test]
    fn project_update_check_uses_configured_binary_and_skips_disabled_managers() {
        let temp = std::env::temp_dir().join(format!(
            "omni-project-update-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp).unwrap();
        let project = temp.join("project");
        fs::create_dir_all(&project).unwrap();
        let binary = temp.join("npm-test");
        fs::write(
            &binary,
            "#!/bin/sh\nprintf '%s\\n' '{\"demo\":{\"current\":\"1.0.0\",\"wanted\":\"1.1.0\",\"latest\":\"2.0.0\"}}'\nexit 1\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&binary).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&binary, permissions).unwrap();

        let mut package = UnifiedPackage::new("demo", "1.0.0", PackageSource::Project);
        package.manager_id = Some(format!("project:npm:{}", project.display()));
        let mut config = ProjectManagersConfig {
            npm_binary: binary.display().to_string(),
            ..ProjectManagersConfig::default()
        };

        let results = check_project_updates(std::slice::from_ref(&package), &config);
        assert_eq!(results.errors.len(), 0);
        assert_eq!(
            results
                .updates
                .get(&ProjectPackageKey {
                    manager_id: package.manager_id.clone().unwrap(),
                    name: "demo".into(),
                })
                .map(String::as_str),
            Some("1.1.0")
        );

        config.npm = false;
        let disabled = check_project_updates(&[package], &config);
        assert!(disabled.updates.is_empty());
        assert!(disabled.errors.is_empty());
        fs::remove_dir_all(temp).unwrap();
    }
}
