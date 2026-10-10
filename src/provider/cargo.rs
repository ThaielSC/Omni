use super::*;
use crate::model::PackageSource;
#[derive(Debug, Default, Clone, Copy)]
pub struct CargoProvider;
pub fn parse_packages(input: &str) -> Vec<UnifiedPackage> {
    let mut out = Vec::new();
    for block in input.split("\n\n") {
        let mut lines = block.lines();
        let Some(head) = lines.next() else { continue };
        let Some((name, rest)) = head.split_once(' ') else {
            continue;
        };
        let version = rest.trim().trim_start_matches("v");
        if version.is_empty() {
            continue;
        }
        let bins = lines
            .map(|l| l.trim().trim_start_matches('*').trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        let mut p = UnifiedPackage::new(name, version, PackageSource::Cargo);
        p.binaries = bins;
        out.push(p);
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
    }
    #[test]
    fn parses_crates2_registry() {
        let p = parse_crates2(r#"{"installs":{"zellij 0.45.1 (registry+x)":{"bins":["zellij"]}}}"#);
        assert_eq!(p[0].name, "zellij");
        assert_eq!(p[0].binaries, vec!["zellij"]);
    }
}
