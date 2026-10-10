/// Formats an optional byte count into a standard human-readable binary IEC string.
///
/// Examples:
/// - `None` => `"Unknown"`
/// - `Some(0)` => `"0 B"`
/// - `Some(1024)` => `"1.00 KiB"`
/// - `Some(5283285)` => `"5.04 MiB"`
/// - `Some(1610612736)` => `"1.50 GiB"`
pub fn format_bytes(bytes: Option<u64>) -> String {
    match bytes {
        None => "Unknown".to_string(),
        Some(0) => "0 B".to_string(),
        Some(b) => {
            const KIB: f64 = 1024.0;
            const MIB: f64 = 1024.0 * KIB;
            const GIB: f64 = 1024.0 * MIB;
            const TIB: f64 = 1024.0 * GIB;

            let bf = b as f64;
            if bf >= TIB {
                format!("{:.2} TiB", bf / TIB)
            } else if bf >= GIB {
                format!("{:.2} GiB", bf / GIB)
            } else if bf >= MIB {
                format!("{:.2} MiB", bf / MIB)
            } else if bf >= KIB {
                format!("{:.2} KiB", bf / KIB)
            } else {
                format!("{} B", b)
            }
        }
    }
}

/// Formats an optional byte count into a compact string (≤ 5 chars) for table rendering.
///
/// Examples:
/// - `None` => `"-"`
/// - `Some(0)` => `"0B"`
/// - `Some(4820000)` => `"4.6M"`
/// - `Some(1200000000)` => `"1.1G"`
pub fn format_bytes_compact(bytes: Option<u64>) -> String {
    match bytes {
        None => "-".to_string(),
        Some(0) => "0B".to_string(),
        Some(b) => {
            const KIB: f64 = 1024.0;
            const MIB: f64 = 1024.0 * KIB;
            const GIB: f64 = 1024.0 * MIB;
            const TIB: f64 = 1024.0 * GIB;

            let bf = b as f64;
            if bf >= TIB {
                format!("{:.1}T", bf / TIB)
            } else if bf >= GIB {
                format!("{:.1}G", bf / GIB)
            } else if bf >= MIB {
                format!("{:.1}M", bf / MIB)
            } else if bf >= KIB {
                format!("{:.1}K", bf / KIB)
            } else {
                format!("{}B", b)
            }
        }
    }
}

/// Parses raw size strings from various package manager outputs into exact bytes.
///
/// Handles:
/// - Raw integer bytes: `"5283285"`
/// - Pacman binary IEC units: `"0.00 B"`, `"284.26 KiB"`, `"6.47 MiB"`, `"1938.36 MiB"`, `"1.5 GiB"`
/// - Flatpak SI units with non-breaking spaces `\u{00A0}`: `"427.9\u{00a0}MB"`, `"442.4\u{00a0}kB"`
/// - Case-insensitive units: `"mb"`, `"MIB"`, `"kb"`, `"kib"`
pub fn parse_size_to_bytes(raw: &str) -> Option<u64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Try parsing as raw integer bytes first
    if let Ok(bytes) = trimmed.parse::<u64>() {
        return Some(bytes);
    }

    // Normalize Unicode non-breaking space (U+00A0) and tabs to standard ASCII space
    let normalized = trimmed.replace('\u{00A0}', " ");
    let mut parts = normalized.split_whitespace();
    let num_str = parts.next()?;
    let unit = parts.next().unwrap_or("B");

    // Reject invalid trailing tokens
    if parts.next().is_some() {
        return None;
    }

    let num: f64 = num_str.parse().ok()?;
    if num < 0.0 || num.is_nan() || num.is_infinite() {
        return None;
    }

    let multiplier: f64 = match unit.to_lowercase().as_str() {
        "b" | "bytes" | "byte" => 1.0,
        "kb" => 1_000.0,
        "kib" => 1_024.0,
        "mb" => 1_000_000.0,
        "mib" => 1_024.0 * 1_024.0,
        "gb" => 1_000_000_000.0,
        "gib" => 1_024.0 * 1_024.0 * 1_024.0,
        "tb" => 1_000_000_000_000.0,
        "tib" => 1_024.0 * 1_024.0 * 1_024.0 * 1_024.0,
        _ => return None,
    };

    let total = num * multiplier;
    if total > u64::MAX as f64 {
        return None;
    }
    Some(total.round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes_none_and_zero() {
        assert_eq!(format_bytes(None), "Unknown");
        assert_eq!(format_bytes(Some(0)), "0 B");
    }

    #[test]
    fn test_format_bytes_boundaries() {
        assert_eq!(format_bytes(Some(512)), "512 B");
        assert_eq!(format_bytes(Some(1024)), "1.00 KiB");
        assert_eq!(format_bytes(Some(1024 * 1024)), "1.00 MiB");
        assert_eq!(format_bytes(Some(5 * 1024 * 1024)), "5.00 MiB");
        assert_eq!(format_bytes(Some(1024 * 1024 * 1024)), "1.00 GiB");
        assert_eq!(format_bytes(Some(2 * 1024 * 1024 * 1024)), "2.00 GiB");
        assert_eq!(format_bytes(Some(1024 * 1024 * 1024 * 1024)), "1.00 TiB");
    }

    #[test]
    fn test_format_bytes_compact() {
        assert_eq!(format_bytes_compact(None), "-");
        assert_eq!(format_bytes_compact(Some(0)), "0B");
        assert_eq!(format_bytes_compact(Some(500)), "500B");
        assert_eq!(format_bytes_compact(Some(5 * 1024 * 1024)), "5.0M");
        assert_eq!(format_bytes_compact(Some(1024 * 1024 * 1024)), "1.0G");
        assert_eq!(format_bytes_compact(Some(1024 * 1024 * 1024 * 1024)), "1.0T");
    }

    #[test]
    fn test_parse_size_plain_integers() {
        assert_eq!(parse_size_to_bytes("0"), Some(0));
        assert_eq!(parse_size_to_bytes("5283285"), Some(5283285));
        assert_eq!(parse_size_to_bytes(" 427912192 "), Some(427912192));
    }

    #[test]
    fn test_parse_size_pacman_units() {
        assert_eq!(parse_size_to_bytes("0.00 B"), Some(0));
        assert_eq!(parse_size_to_bytes("284.26 KiB"), Some(291082));
        assert_eq!(parse_size_to_bytes("6.47 MiB"), Some(6784287));
        assert_eq!(parse_size_to_bytes("1938.36 MiB"), Some(2032517775));
        assert_eq!(parse_size_to_bytes("1.50 GiB"), Some(1610612736));
    }

    #[test]
    fn test_parse_size_flatpak_units_with_nbsp() {
        // \u{00A0} is the non-breaking space emitted by flatpak list
        assert_eq!(parse_size_to_bytes("427.9\u{00a0}MB"), Some(427900000));
        assert_eq!(parse_size_to_bytes("442.4\u{00a0}kB"), Some(442400));
        assert_eq!(parse_size_to_bytes("15.5\u{00a0}MB"), Some(15500000));
        assert_eq!(parse_size_to_bytes("1.2\u{00a0}GB"), Some(1200000000));
    }

    #[test]
    fn test_parse_size_case_insensitivity() {
        assert_eq!(parse_size_to_bytes("10 mib"), Some(10485760));
        assert_eq!(parse_size_to_bytes("10 MIB"), Some(10485760));
        assert_eq!(parse_size_to_bytes("10 mb"), Some(10000000));
        assert_eq!(parse_size_to_bytes("10 bytes"), Some(10));
    }

    #[test]
    fn test_parse_size_invalid_inputs() {
        assert_eq!(parse_size_to_bytes(""), None);
        assert_eq!(parse_size_to_bytes("   "), None);
        assert_eq!(parse_size_to_bytes("invalid"), None);
        assert_eq!(parse_size_to_bytes("-10 MiB"), None);
        assert_eq!(parse_size_to_bytes("10 UnknownUnit"), None);
        assert_eq!(parse_size_to_bytes("10 MiB extra text"), None);
    }
}
