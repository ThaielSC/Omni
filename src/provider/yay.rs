use super::*;
use crate::model::PackageSource;
use std::collections::HashSet;
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
        let foreign: HashSet<String> = parse_packages(&text(out)?)
            .into_iter()
            .map(|package| package.name)
            .collect();
        if foreign.is_empty() {
            return Ok(Vec::new());
        }

        // `-Qm` omits install reasons; `-Qi` provides them. Read package
        // details once rather than launching one subprocess per AUR package.
        let details = run("pacman", &["-Qi"])?;
        Ok(super::pacman::parse_packages(&text(details)?)
            .into_iter()
            .filter_map(|mut package| {
                if foreign.contains(&package.name) {
                    package.source = PackageSource::Aur;
                    Some(package)
                } else {
                    None
                }
            })
            .collect())
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
