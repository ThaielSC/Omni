use super::*;
use crate::model::PackageSource;

#[derive(Debug, Default, Clone, Copy)]
pub struct PacmanProvider;
pub fn parse_packages(input: &str) -> Vec<UnifiedPackage> {
    input
        .split("\n\n")
        .filter_map(|block| {
            let field = |key: &str| {
                block.lines().find_map(|line| {
                    let (k, v) = line.split_once(':')?;
                    (k.trim() == key).then_some(v.trim())
                })
            };
            let name = field("Name")?;
            if name.is_empty() {
                return None;
            }
            let mut p = UnifiedPackage::new(
                name,
                field("Version").unwrap_or("unknown"),
                PackageSource::Pacman,
            );
            if let Some(d) = field("Description") {
                p.description = Some(d.into());
            }
            if let Some(s) = field("Installed Size").and_then(parse_size) {
                p.size_bytes = Some(s);
            }
            if let Some(l) = field("Licenses") {
                p.license = Some(l.into());
            }
            if let Some(url) = field("URL").filter(|s| *s != "None") {
                p.url = Some(url.into());
            }
            if let Some(date) = field("Install Date").filter(|s| *s != "None") {
                p.install_date = Some(date.into());
            }
            if let Some(deps) = field("Depends On").filter(|s| *s != "None") {
                p.dependencies = deps.split_whitespace().map(str::to_owned).collect();
            }
            Some(p)
        })
        .collect()
}
pub(crate) fn parse_size(s: &str) -> Option<u64> {
    let mut parts = s.split_whitespace();
    let n = parts.next()?.parse::<f64>().ok()?;
    let unit = parts.next().unwrap_or("B").to_ascii_lowercase();
    let mult = match unit.as_str() {
        "b" => 1.0,
        "kib" => 1024.0,
        "mib" => 1048576.0,
        "gib" => 1073741824.0,
        "kb" => 1000.0,
        "mb" => 1e6,
        "gb" => 1e9,
        _ => return None,
    };
    Some((n * mult) as u64)
}
impl PackageManagerProvider for PacmanProvider {
    fn id(&self) -> &str {
        "pacman"
    }
    fn is_available(&self) -> bool {
        available("pacman")
    }
    fn list_packages(&self) -> Result<Vec<UnifiedPackage>, ProviderError> {
        let output = run("pacman", &["-Qi"])?;
        Ok(parse_packages(&String::from_utf8_lossy(&output.stdout)))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_qi() {
        let p = parse_packages(
            "Name : nano\nVersion : 8.0-1\nDescription : Editor\nInstalled Size : 2.50 MiB\nLicenses : GPL-3.0\n\n",
        );
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].size_bytes, Some(2_621_440));
        assert_eq!(p[0].description.as_deref(), Some("Editor"));
    }
}
