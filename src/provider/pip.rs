use super::*;
use crate::model::{InstallReason, PackageSource};

#[derive(Debug, Default, Clone, Copy)]
pub struct PipProvider;

pub fn parse_packages(input: &str) -> Result<Vec<UnifiedPackage>, ProviderError> {
    let root: serde_json::Value = serde_json::from_str(input)?;
    if root.is_object()
        && !root
            .get("installed")
            .is_some_and(serde_json::Value::is_array)
    {
        return Err(ProviderError::ParseError(
            "pip inspect JSON is missing its installed array".into(),
        ));
    }
    let rows = if let Some(installed) = root.get("installed").and_then(|v| v.as_array()) {
        installed
            .iter()
            .filter_map(|row| {
                let metadata = row.get("metadata")?;
                let name = metadata.get("name")?.as_str()?;
                let version = metadata.get("version")?.as_str()?;
                let mut package = UnifiedPackage::new(name, version, PackageSource::Pip);

                // pip inspect's `requested` field is the authoritative marker for
                // direct installs. Older/legacy metadata may omit it; leave those
                // packages Unknown rather than inferring from reverse dependencies.
                if let Some(requested) = row.get("requested").and_then(|v| v.as_bool()) {
                    package.install_reason = if requested {
                        InstallReason::Explicit
                    } else {
                        InstallReason::Dependency
                    };
                }
                package.dependencies = metadata
                    .get("requires_dist")
                    .and_then(|v| v.as_array())
                    .into_iter()
                    .flatten()
                    .filter_map(|requirement| requirement.as_str())
                    .filter_map(requirement_name)
                    .collect();
                Some(package)
            })
            .collect::<Vec<_>>()
    } else {
        // Compatibility with `pip list --format=json` and older pip versions.
        // This output does not contain dependency or install-reason metadata.
        root.as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| {
                let name = row.get("name")?.as_str()?;
                let version = row.get("version")?.as_str()?;
                Some(UnifiedPackage::new(name, version, PackageSource::Pip))
            })
            .collect()
    };
    Ok(rows)
}

/// Extract the distribution name from a Requires-Dist (PEP 508) declaration.
/// Requirement names precede extras, version constraints, URLs, and markers.
/// The environment-specific marker is retained only as declared metadata;
/// pip inspect does not report a resolved dependency graph in this field.
fn requirement_name(requirement: &str) -> Option<String> {
    let requirement = requirement.trim();
    let end = requirement
        .find(|ch: char| ch.is_whitespace() || "[<>=!~;".contains(ch))
        .unwrap_or(requirement.len());
    let name = &requirement[..end];
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return None;
    }
    Some(name.to_owned())
}

impl PackageManagerProvider for PipProvider {
    fn id(&self) -> &str {
        "pip"
    }
    fn is_available(&self) -> bool {
        available("pip")
    }
    fn list_packages(&self) -> Result<Vec<UnifiedPackage>, ProviderError> {
        match run("pip", &["inspect"])
            .and_then(text)
            .and_then(|output| parse_packages(&output))
        {
            Ok(packages) => Ok(packages),
            // `pip inspect` was added in pip 22.2. Retain basic inventory support
            // for older pip versions, where dependency provenance is unavailable.
            Err(_) => parse_packages(&text(run("pip", &["list", "--format=json"])?)?),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json() {
        let packages = parse_packages(r#"[{"name":"requests","version":"2.32.0"}]"#).unwrap();
        assert_eq!(packages[0].name, "requests");
        assert_eq!(packages[0].source, PackageSource::Pip);
        assert_eq!(packages[0].install_reason, InstallReason::Unknown);
        assert!(packages[0].dependencies.is_empty());
    }

    #[test]
    fn parses_inspect_provenance_and_declared_dependencies() {
        let input = r#"{
            "version": "1",
            "installed": [
                {"metadata":{"name":"requests","version":"2.32.0","requires_dist":["charset_normalizer<4,>=2","idna<4,>=2.5; extra == 'socks'","urllib3 @ https://example.invalid/urllib3.whl"]},"requested":true},
                {"metadata":{"name":"charset-normalizer","version":"3.4.0","requires_dist":[]},"requested":false},
                {"metadata":{"name":"legacy","version":"1.0"}}
            ]
        }"#;
        let packages = parse_packages(input).unwrap();
        assert_eq!(packages.len(), 3);
        assert_eq!(packages[0].install_reason, InstallReason::Explicit);
        assert_eq!(
            packages[0].dependencies,
            ["charset_normalizer", "idna", "urllib3"]
        );
        assert_eq!(packages[1].install_reason, InstallReason::Dependency);
        assert_eq!(packages[2].install_reason, InstallReason::Unknown);
    }

    #[test]
    fn ignores_malformed_requirement_names() {
        assert_eq!(
            requirement_name("valid_pkg[extra]>=1"),
            Some("valid_pkg".into())
        );
        assert_eq!(requirement_name("@ invalid"), None);
    }
}
