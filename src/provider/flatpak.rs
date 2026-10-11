use super::*;
use crate::model::{InstallReason, PackageSource};
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
                p.install_reason = InstallReason::Explicit;
                p.description = v
                    .get("description")
                    .and_then(|x| x.as_str())
                    .map(str::to_owned);
                p.size_bytes = v
                    .get("installed_size")
                    .and_then(|x| x.as_str())
                    .and_then(super::pacman::parse_size);
                p.url = v.get("origin").and_then(|x| x.as_str()).map(str::to_owned);
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
            p.install_reason = InstallReason::Explicit;
            p.description = Some(c[1].into());
            if let Some(s) = c.get(3).and_then(|x| super::pacman::parse_size(x)) {
                p.size_bytes = Some(s);
            }
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
            &["list", "--app", "--columns=application,name,version,size"],
        )?)?))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_tsv() {
        let p = parse_packages("org.example.App\tExample\t1.2\t12.0 MB");
        assert_eq!(p[0].name, "org.example.App");
        assert_eq!(p[0].size_bytes, Some(12_000_000));
    }
}
