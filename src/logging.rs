//! Small, dependency-free persistent log for user-visible application notices.
//!
//! The logger intentionally records notices and operation summaries rather than
//! subprocess output. Command output can contain environment-specific data, and
//! sudo password input is never passed to this module.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_LOG_BYTES: u64 = 1024 * 1024;
const MAX_ENTRY_CHARS: usize = 2_000;
static LOGGER: OnceLock<Logger> = OnceLock::new();

struct Logger {
    path: PathBuf,
    file: Mutex<File>,
}

impl Logger {
    fn open(path: PathBuf) -> io::Result<Self> {
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "log path has no parent"))?;
        fs::create_dir_all(parent)?;
        set_private_dir_permissions(parent)?;
        rotate_if_needed(&path, 0)?;
        Ok(Self {
            file: Mutex::new(open_append(&path)?),
            path,
        })
    }

    fn append(&self, level: &str, message: &str) -> io::Result<()> {
        let mut file = self
            .file
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let line = format!("{} [{}] {}\n", timestamp_utc(), level, one_line(message));
        let current_len = file.metadata()?.len();
        if current_len.saturating_add(line.len() as u64) > MAX_LOG_BYTES {
            rotate_if_needed(&self.path, current_len)?;
            *file = open_append(&self.path)?;
        }
        file.write_all(line.as_bytes())?;
        file.flush()
    }
}

/// Initialize persistent logging. Safe to call more than once.
pub fn initialize() -> io::Result<PathBuf> {
    if let Some(logger) = LOGGER.get() {
        return Ok(logger.path.clone());
    }
    let path = default_log_path()?;
    let logger = Logger::open(path)?;
    let _ = LOGGER.set(logger);
    Ok(LOGGER.get().expect("logger was initialized").path.clone())
}

/// The configured log file path, even if logging has not been initialized yet.
pub fn path() -> io::Result<PathBuf> {
    LOGGER
        .get()
        .map(|logger| logger.path.clone())
        .map_or_else(default_log_path, Ok)
}

/// Append a user-visible informational event when logging is available.
pub fn info(message: &str) {
    write("INFO", message);
}

/// Append a user-visible warning when logging is available.
pub fn warn(message: &str) {
    write("WARN", message);
}

/// Append an application error when logging is available.
pub fn error(message: &str) {
    write("ERROR", message);
}

fn write(level: &str, message: &str) {
    if let Some(logger) = LOGGER.get() {
        let _ = logger.append(level, message);
    }
}

/// Return the most recent log lines, preserving their original order.
pub fn read_recent(max_lines: usize) -> io::Result<String> {
    let path = path()?;
    let mut lines = Vec::new();
    for candidate in [rotated_path(&path), path] {
        let file = match File::open(candidate) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        lines.extend(
            BufReader::new(file)
                .lines()
                .collect::<io::Result<Vec<_>>>()?,
        );
    }
    let skip = lines.len().saturating_sub(max_lines);
    Ok(lines.into_iter().skip(skip).collect::<Vec<_>>().join("\n"))
}

fn default_log_path() -> io::Result<PathBuf> {
    if let Some(state_home) = std::env::var_os("XDG_STATE_HOME") {
        let root = PathBuf::from(state_home);
        if root.is_absolute() {
            return Ok(root.join("omni/omni.log"));
        }
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".local/state/omni/omni.log"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "neither XDG_STATE_HOME nor HOME is set",
            )
        })
}

fn open_append(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path)?;
    set_private_file_permissions(path)?;
    Ok(file)
}

fn rotate_if_needed(path: &Path, current_len: u64) -> io::Result<()> {
    if current_len < MAX_LOG_BYTES || !path.exists() {
        return Ok(());
    }
    let rotated = rotated_path(path);
    match fs::remove_file(&rotated) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    fs::rename(path, &rotated)?;
    set_private_file_permissions(&rotated)
}

fn rotated_path(path: &Path) -> PathBuf {
    path.with_extension("log.1")
}

fn timestamp_utc() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (seconds / 86_400) as i64;
    let day_seconds = seconds % 86_400;

    // Gregorian calendar conversion (days since Unix epoch), avoiding a date
    // dependency just for log timestamps.
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);

    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        day_seconds / 3_600,
        (day_seconds % 3_600) / 60,
        day_seconds % 60
    )
}

fn one_line(message: &str) -> String {
    message
        .chars()
        .take(MAX_ENTRY_CHARS)
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(unix)]
fn set_private_dir_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_private_dir_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_single_line_entries_and_reads_latest_lines() {
        let directory = tempfile::tempdir().unwrap();
        let logger = Logger::open(directory.path().join("omni.log")).unwrap();
        logger.append("INFO", "first event\nsecond line").unwrap();
        logger.append("WARN", "second event").unwrap();

        let contents = fs::read_to_string(directory.path().join("omni.log")).unwrap();
        assert_eq!(contents.lines().count(), 2);
        assert!(contents.contains("[INFO] first event second line"));
        assert!(contents.contains("[WARN] second event"));
        assert!(contents.starts_with("20"));
        assert!(contents.contains("T") && contents.contains("Z [INFO]"));
    }

    #[test]
    fn rotates_large_log_before_appending() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("omni.log");
        fs::write(&path, vec![b'x'; MAX_LOG_BYTES as usize]).unwrap();
        let logger = Logger::open(path.clone()).unwrap();
        logger.append("INFO", "after rotation").unwrap();

        assert!(rotated_path(&path).exists());
        assert!(fs::read_to_string(path).unwrap().contains("after rotation"));
    }
}
