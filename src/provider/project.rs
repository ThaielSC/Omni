//! Discovery of direct dependencies declared by the project containing the
//! current directory. These are manifest entries, not globally installed
//! packages, so their size and resolved version are usually unknown.

use crate::model::{InstallReason, PackageSource, UnifiedPackage};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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

/// Find project managers for the nearest Cargo project and Node project(s).
/// Each Node workspace package gets its own manager context so package actions
/// run from the manifest that declares the dependency. A Bun lockfile or
/// `packageManager: "bun@..."` makes Node workspace entries use Bun.
///
/// The function is filesystem-only: it does not execute package-manager
/// commands, so it is safe to call during initial inventory loading.
pub fn discover_projects(start: impl AsRef<Path>) -> Vec<ProjectContext> {
    let start = start.as_ref();
    let canonical = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    let directory = if canonical.is_file() {
        canonical.parent().unwrap_or(&canonical)
    } else {
        canonical.as_path()
    };

    let mut contexts = Vec::new();
    if let Some(root) = directory
        .ancestors()
        .find(|root| root.join("Cargo.toml").is_file())
    {
        if let Ok(contents) = std::fs::read_to_string(root.join("Cargo.toml")) {
            let workspace = find_cargo_workspace_root(root)
                .filter(|workspace_root| *workspace_root != root)
                .and_then(|workspace_root| {
                    std::fs::read_to_string(workspace_root.join("Cargo.toml")).ok()
                });
            let packages = parse_cargo_dependencies_with_workspace(&contents, workspace.as_deref());
            contexts.push(ProjectContext::new(root, "cargo", packages));
        }
    }

    let nearest_node = directory
        .ancestors()
        .find(|root| root.join("package.json").is_file());
    if let Some(package_root) = nearest_node {
        let workspace_root = find_node_workspace_root(package_root);
        let root = workspace_root.unwrap_or(package_root);
        let root_manifest = read_node_manifest(root);
        let ecosystem = if is_bun_project(root, &root_manifest) {
            "bun"
        } else {
            "npm"
        };
        let package_roots = workspace_root
            .and_then(|workspace_root| node_workspace_members(workspace_root, &root_manifest))
            .unwrap_or_else(|| vec![package_root.to_path_buf()]);
        for package_root in package_roots {
            let manifest = read_node_manifest(&package_root);
            contexts.push(ProjectContext::new(
                &package_root,
                ecosystem,
                parse_node_dependencies(&manifest),
            ));
        }
    }
    contexts
}

/// Parse direct dependencies from Cargo manifest tables. If a dependency uses
/// `workspace = true`, its version is resolved from `[workspace.dependencies]`
/// when that table is present in the input. Discovery also supplies a separate
/// ancestor workspace manifest for Cargo's usual split-manifest layout.
pub fn parse_cargo_dependencies(input: &str) -> Vec<UnifiedPackage> {
    let Ok(manifest) = input.parse::<toml::Value>() else {
        return Vec::new();
    };
    parse_cargo_manifest(&manifest, manifest.get("workspace"))
}

fn parse_cargo_dependencies_with_workspace(
    input: &str,
    workspace_input: Option<&str>,
) -> Vec<UnifiedPackage> {
    let Ok(manifest) = input.parse::<toml::Value>() else {
        return Vec::new();
    };
    let workspace_manifest = workspace_input.and_then(|input| input.parse::<toml::Value>().ok());
    let workspace = workspace_manifest
        .as_ref()
        .and_then(|value| value.get("workspace"))
        .or_else(|| manifest.get("workspace"));
    parse_cargo_manifest(&manifest, workspace)
}

fn parse_cargo_manifest(
    manifest: &toml::Value,
    workspace: Option<&toml::Value>,
) -> Vec<UnifiedPackage> {
    let workspace_dependencies = workspace
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table);
    let mut packages = Vec::new();
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(dependencies) = manifest.get(section).and_then(toml::Value::as_table) {
            packages.extend(dependencies.iter().map(|(name, spec)| {
                let inherited = spec
                    .as_table()
                    .and_then(|fields| fields.get("workspace"))
                    .and_then(toml::Value::as_bool)
                    .unwrap_or(false);
                let effective_spec = if inherited {
                    workspace_dependencies.and_then(|dependencies| dependencies.get(name))
                } else {
                    Some(spec)
                };
                let version = match effective_spec {
                    Some(toml::Value::String(version)) => version.clone(),
                    Some(toml::Value::Table(fields)) => fields
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

fn find_cargo_workspace_root(project_root: &Path) -> Option<&Path> {
    project_root.ancestors().find(|ancestor| {
        std::fs::read_to_string(ancestor.join("Cargo.toml"))
            .ok()
            .and_then(|contents| contents.parse::<toml::Value>().ok())
            .is_some_and(|manifest| manifest.get("workspace").is_some())
    })
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

fn find_node_workspace_root(package_root: &Path) -> Option<&Path> {
    package_root.ancestors().find(|ancestor| {
        let manifest = read_node_manifest(ancestor);
        workspace_patterns(&manifest).is_some()
    })
}

fn read_node_manifest(root: &Path) -> serde_json::Value {
    std::fs::read_to_string(root.join("package.json"))
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or(serde_json::Value::Null)
}

fn workspace_patterns(manifest: &serde_json::Value) -> Option<Vec<String>> {
    let workspaces = manifest.get("workspaces")?;
    let patterns = workspaces
        .as_array()
        .or_else(|| workspaces.get("packages")?.as_array())?;
    Some(
        patterns
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_owned)
            .collect(),
    )
}

fn node_workspace_members(
    workspace_root: &Path,
    manifest: &serde_json::Value,
) -> Option<Vec<PathBuf>> {
    let patterns = workspace_patterns(manifest)?;
    let includes: Vec<_> = patterns
        .iter()
        .filter(|pattern| !pattern.starts_with('!'))
        .collect();
    let excludes: Vec<_> = patterns
        .iter()
        .filter_map(|pattern| pattern.strip_prefix('!'))
        .collect();
    let mut members = Vec::new();
    if workspace_root.join("package.json").is_file() {
        members.push(workspace_root.to_path_buf());
    }

    for entry in WalkDir::new(workspace_root)
        .follow_links(false)
        .max_depth(10)
        .into_iter()
        .filter_entry(|entry| {
            !entry.file_name().to_string_lossy().starts_with('.')
                && !matches!(
                    entry.file_name().to_str(),
                    Some("node_modules" | "target" | "dist" | ".yarn")
                )
        })
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && entry.file_name() == "package.json")
    {
        let Some(member_root) = entry.path().parent() else {
            continue;
        };
        if member_root == workspace_root {
            continue;
        }
        let Ok(relative) = member_root.strip_prefix(workspace_root) else {
            continue;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        let included = includes
            .iter()
            .any(|pattern| workspace_pattern_matches(pattern, &relative));
        let excluded = excludes
            .iter()
            .any(|pattern| workspace_pattern_matches(pattern, &relative));
        if included && !excluded {
            members.push(member_root.to_path_buf());
        }
    }
    members.sort();
    members.dedup();
    Some(members)
}

fn workspace_pattern_matches(pattern: &str, path: &str) -> bool {
    let pattern_parts: Vec<_> = pattern.trim_matches('/').split('/').collect();
    let path_parts: Vec<_> = path.trim_matches('/').split('/').collect();
    workspace_path_parts_match(&pattern_parts, &path_parts)
}

fn workspace_path_parts_match(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => {
            workspace_path_parts_match(rest, path)
                || path
                    .split_first()
                    .is_some_and(|(_, remaining)| workspace_path_parts_match(pattern, remaining))
        }
        Some((component, rest)) => path.split_first().is_some_and(|(part, remaining)| {
            wildcard_component_matches(component, part)
                && workspace_path_parts_match(rest, remaining)
        }),
    }
}

fn wildcard_component_matches(pattern: &str, value: &str) -> bool {
    let pattern = pattern.as_bytes();
    let value = value.as_bytes();
    let (mut p, mut v, mut star, mut retry) = (0, 0, None, 0);
    while v < value.len() {
        if p < pattern.len() && (pattern[p] == b'?' || pattern[p] == value[v]) {
            p += 1;
            v += 1;
        } else if p < pattern.len() && pattern[p] == b'*' {
            star = Some(p);
            p += 1;
            retry = v;
        } else if let Some(star_at) = star {
            p = star_at + 1;
            retry += 1;
            v = retry;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }
    p == pattern.len()
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
