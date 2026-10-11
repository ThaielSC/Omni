//! Package discovery adapters. Missing executables are reported as `NotAvailable`.
use crate::model::{ProviderError, UnifiedPackage};
use std::process::{Command, Output};

pub mod cargo;
pub mod custom;
pub mod flatpak;
pub mod pacman;
pub mod pip;
pub mod uninstall;
pub mod uv;
pub mod yay;

pub use uninstall::{UninstallCommand, build_uninstall_command};

pub trait PackageManagerProvider: Send + Sync {
    fn id(&self) -> &str;
    fn is_available(&self) -> bool;
    fn list_packages(&self) -> Result<Vec<UnifiedPackage>, ProviderError>;
}

pub(crate) fn run(binary: &str, args: &[&str]) -> Result<Output, ProviderError> {
    let output = Command::new(binary).args(args).output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ProviderError::NotAvailable(binary.into())
        } else {
            ProviderError::Io(e)
        }
    })?;
    if !output.status.success() {
        return Err(ProviderError::CommandFailed(format!(
            "{binary} {}",
            args.join(" ")
        )));
    }
    Ok(output)
}

pub(crate) fn available(binary: &str) -> bool {
    if binary.contains(std::path::MAIN_SEPARATOR) {
        return std::path::Path::new(binary).is_file();
    }
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| {
            let path = dir.join(binary);
            path.is_file() && {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::metadata(path).is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
                }
                #[cfg(not(unix))]
                {
                    true
                }
            }
        })
    })
}

pub(crate) fn text(output: Output) -> Result<String, ProviderError> {
    String::from_utf8(output.stdout).map_err(|e| ProviderError::ParseError(e.to_string()))
}
