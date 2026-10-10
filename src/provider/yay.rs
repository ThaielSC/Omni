use super::*;
use crate::model::PackageSource;
#[derive(Debug, Default, Clone, Copy)]
pub struct YayProvider;
pub fn parse_packages(input: &str) -> Vec<UnifiedPackage> {
    input
        .lines()
        .filter_map(|l| {
            let mut x = l.split_whitespace();
            let n = x.next()?;
            let v = x.next()?;
            if n.is_empty() {
                return None;
            }
            let mut p = UnifiedPackage::new(n, v, PackageSource::Aur);
            if let Some(s) = x
                .next()
                .and_then(|s| s.trim_matches(['[', ']']).parse().ok())
            {
                p.size_bytes = Some(s);
            }
            Some(p)
        })
        .collect()
}
impl PackageManagerProvider for YayProvider {
    fn id(&self) -> &str {
        "yay"
    }
    fn is_available(&self) -> bool {
        available("pacman")
    }
    fn list_packages(&self) -> Result<Vec<UnifiedPackage>, ProviderError> {
        let out = run("pacman", &["-Qm"])?;
        let mut p = parse_packages(&text(out)?);
        for x in &mut p {
            x.source = PackageSource::Aur;
        }
        Ok(p)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_foreign() {
        let p = parse_packages("yay-bin 12.3-1\nfoo 2.0-4");
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].source, PackageSource::Aur);
        assert_eq!(p[0].version, "12.3-1");
    }
}
