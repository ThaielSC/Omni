use super::*;
use crate::model::PackageSource;
use serde::Deserialize;
#[derive(Debug, Default, Clone, Copy)]
pub struct PipProvider;
#[derive(Deserialize)]
struct PipRow {
    name: String,
    version: String,
}
pub fn parse_packages(input: &str) -> Result<Vec<UnifiedPackage>, ProviderError> {
    let rows: Vec<PipRow> = serde_json::from_str(input)?;
    Ok(rows
        .into_iter()
        .map(|r| UnifiedPackage::new(r.name, r.version, PackageSource::Pip))
        .collect())
}
impl PackageManagerProvider for PipProvider {
    fn id(&self) -> &str {
        "pip"
    }
    fn is_available(&self) -> bool {
        available("pip")
    }
    fn list_packages(&self) -> Result<Vec<UnifiedPackage>, ProviderError> {
        parse_packages(&text(run("pip", &["list", "--format=json"])?)?)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_json() {
        let p = parse_packages(r#"[{"name":"requests","version":"2.32.0"}]"#).unwrap();
        assert_eq!(p[0].name, "requests");
        assert_eq!(p[0].source, PackageSource::Pip);
    }
}
