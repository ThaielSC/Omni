use super::*;
use crate::model::{InstallReason, PackageSource};
use std::process::Command;

#[derive(Debug, Default, Clone, Copy)]
pub struct PacmanProvider;

fn parse_query_output(output: std::process::Output) -> Result<Vec<UnifiedPackage>, ProviderError> {
    // pacman exits with status 1 and no output when the local database is
    // empty. Treat that specific response as an empty inventory; other
    // failures still need to surface as provider errors.
    if output.status.success() {
        Ok(parse_packages(&String::from_utf8_lossy(&output.stdout)))
    } else if output.status.code() == Some(1)
        && output.stdout.is_empty()
        && output.stderr.is_empty()
    {
        Ok(Vec::new())
    } else {
        Err(ProviderError::CommandFailed("pacman -Qi".into()))
    }
}

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
            p.install_reason = field("Install Reason")
                .and_then(parse_install_reason)
                .unwrap_or(InstallReason::Unknown);
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

/// Parse pacman's stable English `pacman -Qi` install-reason field.
fn parse_install_reason(value: &str) -> Option<InstallReason> {
    match value.trim() {
        "Explicitly installed" => Some(InstallReason::Explicit),
        "Installed as a dependency for another package" => Some(InstallReason::Dependency),
        _ => None,
    }
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
        let output = Command::new("pacman").arg("-Qi").output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ProviderError::NotAvailable("pacman".into())
            } else {
                ProviderError::Io(e)
            }
        })?;

        parse_query_output(output)
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

    #[test]
    fn accepts_pacman_empty_database_response() {
        use std::os::unix::process::ExitStatusExt;

        let output = std::process::Output {
            status: std::process::ExitStatus::from_raw(1 << 8),
            stdout: Vec::new(),
            stderr: Vec::new(),
        };
        assert!(parse_query_output(output).unwrap().is_empty());
    }

    #[test]
    fn does_not_suppress_pacman_errors_with_diagnostics() {
        use std::os::unix::process::ExitStatusExt;

        let output = std::process::Output {
            status: std::process::ExitStatus::from_raw(1 << 8),
            stdout: Vec::new(),
            stderr: b"database is locked".to_vec(),
        };
        assert!(parse_query_output(output).is_err());
    }
}
