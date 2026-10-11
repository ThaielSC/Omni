use super::*;
use crate::model::{InstallReason, PackageSource};
#[derive(Debug, Default, Clone, Copy)]
pub struct CargoProvider;
pub fn parse_packages(input: &str) -> Vec<UnifiedPackage> {
    let mut out = Vec::new();
    let mut current: Option<UnifiedPackage> = None;
    for line in input.lines() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        let indented = line.starts_with(' ') || line.starts_with('\t');
        if !indented
            && let Some(header) = line.strip_suffix(':')
            && let Some((name, version)) = header.split_once(' ')
            && let Some(version) = version.trim().strip_prefix('v')
            && !name.is_empty()
            && !version.is_empty()
        {
            if let Some(package) = current.take() {
                out.push(package);
            }
            let mut package = UnifiedPackage::new(name, version, PackageSource::Cargo);
            package.install_reason = InstallReason::Explicit;
            current = Some(package);
        } else if indented && let Some(package) = current.as_mut() {
            let binary = line.trim().trim_start_matches('*').trim();
            if !binary.is_empty() {
                package.binaries.push(binary.to_owned());
            }
        }
    }
    if let Some(package) = current {
        out.push(package);
    }
    out
}
impl PackageManagerProvider for CargoProvider {
    fn id(&self) -> &str {
        "cargo"
    }
    fn is_available(&self) -> bool {
        available("cargo")
    }
    fn list_packages(&self) -> Result<Vec<UnifiedPackage>, ProviderError> {
        match run("cargo", &["install", "--list"]) {
            Ok(output) => Ok(parse_packages(&String::from_utf8_lossy(&output.stdout))),
            Err(error) => {
                let home = std::env::var_os("CARGO_HOME")
                    .map(std::path::PathBuf::from)
                    .or_else(|| {
                        std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".cargo"))
                    });
                if let Some(home) = home {
                    if let Ok(contents) = std::fs::read_to_string(home.join(".crates2.json")) {
                        return Ok(parse_crates2(&contents));
                    }
                }
                Err(error)
            }
        }
    }
}
pub fn parse_crates2(input: &str) -> Vec<UnifiedPackage> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(input) else {
        return Vec::new();
    };
    root.pointer("/installs")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(key, val)| {
            let mut fields = key.split_whitespace();
            let name = fields.next()?;
            let version = fields.next()?;
            let mut package = UnifiedPackage::new(name, version, PackageSource::Cargo);
            package.install_reason = InstallReason::Explicit;
            package.binaries = val
                .get("bins")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|bin| bin.as_str().map(str::to_owned))
                .collect();
            Some(package)
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_install_list() {
        let p = parse_packages("ripgrep v14.1.0:\n    rg\n\nfd v10.2.0:\n    fd");
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].binaries, vec!["rg"]);
        assert_eq!(p[0].version, "14.1.0");
        assert_eq!(p[1].version, "10.2.0");
    }

    #[test]
    fn parses_adjacent_install_headers_without_blank_lines() {
        let packages = parse_packages(
            "cargo-update v22.1.1:\n    cargo-install-update\n    cargo-install-update-config\nrustfilt v0.2.0:\n    rustfilt\n",
        );
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "cargo-update");
        assert_eq!(packages[0].version, "22.1.1");
        assert_eq!(
            packages[0].binaries,
            ["cargo-install-update", "cargo-install-update-config"]
        );
        assert_eq!(packages[1].name, "rustfilt");
        assert_eq!(packages[1].version, "0.2.0");
        assert_eq!(packages[1].binaries, ["rustfilt"]);
    }
    #[test]
    fn parses_crates2_registry() {
        let p = parse_crates2(r#"{"installs":{"zellij 0.45.1 (registry+x)":{"bins":["zellij"]}}}"#);
        assert_eq!(p[0].name, "zellij");
        assert_eq!(p[0].binaries, vec!["zellij"]);
    }
}
