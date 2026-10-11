use super::*;
use crate::model::PackageSource;
#[derive(Debug, Default, Clone, Copy)]
pub struct FlatpakProvider;
pub fn parse_packages(input: &str) -> Vec<UnifiedPackage> {
    if input.trim_start().starts_with('[') {
        return serde_json::from_str::<Vec<serde_json::Value>>(input)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| {
                let name = v
                    .get("application_id")
                    .or_else(|| v.get("application"))
                    .and_then(|x| x.as_str())?;
                let version = v
                    .get("version")
                    .and_then(|x| x.as_str())
                    .unwrap_or("unknown");
                let mut p = UnifiedPackage::new(name, version, PackageSource::Flatpak);
                p.description = v
                    .get("description")
                    .and_then(|x| x.as_str())
                    .map(str::to_owned);
                p.size_bytes = v
                    .get("installed_size")
                    .and_then(|x| x.as_str())
                    .and_then(super::pacman::parse_size);
                p.url = v.get("origin").and_then(|x| x.as_str()).map(str::to_owned);
                // Flatpak reports the exact runtime ref required by the app, but
                // `flatpak list` does not reveal whether it was installed
                // explicitly or pulled in as a dependency.
                p.dependencies = v
                    .get("runtime")
                    .and_then(|x| x.as_str())
                    .filter(|runtime| !runtime.is_empty())
                    .map(|runtime| vec![runtime.to_owned()])
                    .unwrap_or_default();
                Some(p)
            })
            .collect();
    }
    input
        .lines()
        .filter_map(|l| {
            let c: Vec<_> = l.split('\t').collect();
            if c.len() < 3 || c[0].is_empty() {
                return None;
            }
            let mut p = UnifiedPackage::new(c[0], c[2], PackageSource::Flatpak);
            p.description = Some(c[1].into());
            if let Some(s) = c.get(3).and_then(|x| super::pacman::parse_size(x)) {
                p.size_bytes = Some(s);
            }
            p.dependencies = c
                .get(4)
                .filter(|runtime| !runtime.is_empty())
                .map(|runtime| vec![(*runtime).to_owned()])
                .unwrap_or_default();
            Some(p)
        })
        .collect()
}
impl PackageManagerProvider for FlatpakProvider {
    fn id(&self) -> &str {
        "flatpak"
    }
    fn is_available(&self) -> bool {
        available("flatpak")
    }
    fn list_packages(&self) -> Result<Vec<UnifiedPackage>, ProviderError> {
        Ok(parse_packages(&text(run(
            "flatpak",
            &["list", "--app", "--json"],
        )?)?))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_tsv() {
        let p = parse_packages(
            "org.example.App\tExample\t1.2\t12.0 MB\torg.freedesktop.Platform/x86_64/24.08",
        );
        assert_eq!(p[0].name, "org.example.App");
        assert_eq!(p[0].size_bytes, Some(12_000_000));
        assert_eq!(p[0].dependencies, ["org.freedesktop.Platform/x86_64/24.08"]);
        assert_eq!(p[0].install_reason, crate::model::InstallReason::Unknown);
    }

    #[test]
    fn parses_json_runtime_dependency_without_guessing_install_reason() {
        let p = parse_packages(
            r#"[{"application_id":"org.example.App","name":"Example","version":"1.2","runtime":"org.freedesktop.Platform/x86_64/24.08"}]"#,
        );
        assert_eq!(p[0].dependencies, ["org.freedesktop.Platform/x86_64/24.08"]);
        assert_eq!(p[0].install_reason, crate::model::InstallReason::Unknown);
    }
}
