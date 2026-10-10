use super::*;
use crate::model::PackageSource;
#[derive(Debug, Default, Clone, Copy)]
pub struct UvProvider;
pub fn parse_packages(input: &str) -> Vec<UnifiedPackage> {
    input
        .lines()
        .filter_map(|l| {
            let line = l.trim();
            if line.is_empty() || line.starts_with("- ") {
                return None;
            }
            let (name, rest) = line.split_once(' ')?;
            let version = rest
                .trim()
                .trim_start_matches('v')
                .split_whitespace()
                .next()?;
            let version = version.trim_matches(['(', ')']);
            if version.is_empty() {
                return None;
            }
            Some(UnifiedPackage::new(name, version, PackageSource::Uv))
        })
        .collect()
}
impl PackageManagerProvider for UvProvider {
    fn id(&self) -> &str {
        "uv"
    }
    fn is_available(&self) -> bool {
        available("uv")
    }
    fn list_packages(&self) -> Result<Vec<UnifiedPackage>, ProviderError> {
        Ok(parse_packages(&text(run("uv", &["tool", "list"])?)?))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_uv() {
        let p = parse_packages("ruff v0.9.1\nblack v24.10.0 (Python 3.12)");
        assert_eq!(p.len(), 2);
        assert_eq!(p[1].version, "24.10.0");
    }
}
