//! Discovery of direct dependencies declared by the project containing the
//! current directory. These are manifest entries, not globally installed
//! packages, so their size and resolved version are usually unknown.

use crate::model::{InstallReason, PackageSource, UnifiedPackage};
use std::path::{Path, PathBuf};

/// A project-local package manager represented in the manager pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContext {
    /// Canonical project root containing the manifest.
    pub root: PathBuf,
    /// Manager identifier, such as `cargo`, `npm`, or `bun`.
    pub ecosystem: &'static str,
    /// Display label in the manager pane, e.g. `my-app[npm]`.
    pub label: String,
    /// Stable identity for packages, including project root.
    pub manager_id: String,
    /// Direct dependencies declared by the project's manifest.
    pub packages: Vec<UnifiedPackage>,
}

impl ProjectContext {
    fn new(root: &Path, ecosystem: &'static str, packages: Vec<UnifiedPackage>) -> Self {
        let folder = root
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("project");
        let manager_id = format!("project:{ecosystem}:{}", root.display());
        let display_ecosystem = match ecosystem {
            "cargo" => "Cargo",
            "npm" => "npm",
            "bun" => "bun",
            _ => ecosystem,
        };
        let label = format!("{folder}[{display_ecosystem}]");
        let packages = packages
            .into_iter()
            .map(|mut package| {
                package.manager_id = Some(manager_id.clone());
                package
            })
            .collect();

        Self {
            root: root.to_path_buf(),
            ecosystem,
            label,
            manager_id,
            packages,
        }
    }
}

/// Find the nearest ancestor project containing Cargo and/or Node manifests.
/// A monorepo root may expose both a Cargo and a Node manager entry. A Bun
/// lockfile or `packageManager: "bun@..."` makes the Node entry use Bun.
///
/// The function is intentionally filesystem-only: it does not execute package
/// manager commands, so it is safe to call during initial inventory loading.
pub fn discover_projects(start: impl AsRef<Path>) -> Vec<ProjectContext> {
    let start = start.as_ref();
    let canonical = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    let directory = if canonical.is_file() {
        canonical.parent().unwrap_or(&canonical)
    } else {
        canonical.as_path()
    };

    for root in directory.ancestors() {
        let cargo_manifest = root.join("Cargo.toml");
        let node_manifest = root.join("package.json");
        let has_cargo = cargo_manifest.is_file();
        let has_node = node_manifest.is_file();
        if !has_cargo && !has_node {
            continue;
        }

        let mut contexts = Vec::new();
        if has_cargo {
            if let Ok(contents) = std::fs::read_to_string(&cargo_manifest) {
                let packages = parse_cargo_dependencies(&contents);
                contexts.push(ProjectContext::new(root, "cargo", packages));
            }
        }
        if has_node {
            if let Ok(contents) = std::fs::read_to_string(&node_manifest) {
                let value: serde_json::Value =
                    serde_json::from_str(&contents).unwrap_or(serde_json::Value::Null);
                let ecosystem = if is_bun_project(root, &value) {
                    "bun"
                } else {
                    "npm"
                };
                let packages = parse_node_dependencies(&value);
                contexts.push(ProjectContext::new(root, ecosystem, packages));
            }
        }
        return contexts;
    }
    Vec::new()
}

/// Parse direct dependencies from Cargo manifest tables. Workspace-inherited
/// dependencies are listed with `*` because the actual version lives in the
/// workspace root's `[workspace.dependencies]` table.
pub fn parse_cargo_dependencies(input: &str) -> Vec<UnifiedPackage> {
    let Ok(manifest) = input.parse::<toml::Value>() else {
        return Vec::new();
    };
    let mut packages = Vec::new();
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(dependencies) = manifest.get(section).and_then(toml::Value::as_table) {
            packages.extend(dependencies.iter().map(|(name, spec)| {
                let version = match spec {
                    toml::Value::String(version) => version.clone(),
                    toml::Value::Table(fields) => fields
                        .get("version")
                        .and_then(toml::Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| "*".to_owned()),
                    _ => "*".to_owned(),
                };
                project_package(name, version, "cargo", section)
            }));
        }
    }
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    packages.dedup_by(|a, b| a.name == b.name);
    packages
}

/// Parse the direct dependencies in a Node `package.json`, preserving the
/// manifest range as the displayed version.
pub fn parse_node_dependencies(manifest: &serde_json::Value) -> Vec<UnifiedPackage> {
    let mut packages = Vec::new();
    for (section, description) in [
        ("dependencies", "runtime dependency"),
        ("devDependencies", "development dependency"),
        ("optionalDependencies", "optional dependency"),
        ("peerDependencies", "peer dependency"),
    ] {
        if let Some(dependencies) = manifest.get(section).and_then(serde_json::Value::as_object) {
            packages.extend(dependencies.iter().map(|(name, version)| {
                project_package(
                    name,
                    version.as_str().unwrap_or("*").to_owned(),
                    "npm",
                    description,
                )
            }));
        }
    }
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    packages.dedup_by(|a, b| a.name == b.name);
    packages
}

fn project_package(name: &str, version: String, ecosystem: &str, category: &str) -> UnifiedPackage {
    UnifiedPackage::new(name, version, PackageSource::Project)
        .with_install_reason(InstallReason::Explicit)
        .with_description(format!("{category} in {ecosystem} project"))
}

fn is_bun_project(root: &Path, package_json: &serde_json::Value) -> bool {
    root.join("bun.lock").is_file()
        || root.join("bun.lockb").is_file()
        || package_json
            .get("packageManager")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|manager| manager.starts_with("bun@"))
}
