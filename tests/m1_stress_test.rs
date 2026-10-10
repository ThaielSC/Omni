//! Milestone 1 Adversarial Stress Test Suite
//! Empirical verification of models, formatters, and utilities.

use omni::model::{PackageSource, SortMode, UnifiedPackage};
use omni::utils::format::{format_bytes, format_bytes_compact, parse_size_to_bytes};

// =============================================================================
// CHALLENGE SUITE 1: parse_size_to_bytes
// =============================================================================

#[test]
fn test_stress_parse_size_all_units() {
    // IEC binary units
    assert_eq!(parse_size_to_bytes("1 B"), Some(1));
    assert_eq!(parse_size_to_bytes("1 Byte"), Some(1));
    assert_eq!(parse_size_to_bytes("1 Bytes"), Some(1));
    assert_eq!(parse_size_to_bytes("1 KiB"), Some(1024));
    assert_eq!(parse_size_to_bytes("1 MiB"), Some(1024 * 1024));
    assert_eq!(parse_size_to_bytes("1 GiB"), Some(1024 * 1024 * 1024));
    assert_eq!(parse_size_to_bytes("1 TiB"), Some(1024 * 1024 * 1024 * 1024));

    // SI decimal units
    assert_eq!(parse_size_to_bytes("1 kB"), Some(1000));
    assert_eq!(parse_size_to_bytes("1 MB"), Some(1000 * 1000));
    assert_eq!(parse_size_to_bytes("1 GB"), Some(1000 * 1000 * 1000));
    assert_eq!(parse_size_to_bytes("1 TB"), Some(1000 * 1000 * 1000 * 1000));

    // Case insensitivity
    assert_eq!(parse_size_to_bytes("5 kib"), Some(5 * 1024));
    assert_eq!(parse_size_to_bytes("5 KIB"), Some(5 * 1024));
    assert_eq!(parse_size_to_bytes("5 mib"), Some(5 * 1024 * 1024));
    assert_eq!(parse_size_to_bytes("5 MIB"), Some(5 * 1024 * 1024));
    assert_eq!(parse_size_to_bytes("5 gib"), Some(5 * 1024 * 1024 * 1024));
    assert_eq!(parse_size_to_bytes("5 GIB"), Some(5 * 1024 * 1024 * 1024));
    assert_eq!(parse_size_to_bytes("5 tib"), Some(5 * 1024 * 1024 * 1024 * 1024));
    assert_eq!(parse_size_to_bytes("5 TIB"), Some(5 * 1024 * 1024 * 1024 * 1024));
    assert_eq!(parse_size_to_bytes("5 kb"), Some(5000));
    assert_eq!(parse_size_to_bytes("5 KB"), Some(5000));
    assert_eq!(parse_size_to_bytes("5 mb"), Some(5_000_000));
    assert_eq!(parse_size_to_bytes("5 MB"), Some(5_000_000));
    assert_eq!(parse_size_to_bytes("5 gb"), Some(5_000_000_000));
    assert_eq!(parse_size_to_bytes("5 GB"), Some(5_000_000_000));
    assert_eq!(parse_size_to_bytes("5 tb"), Some(5_000_000_000_000));
    assert_eq!(parse_size_to_bytes("5 TB"), Some(5_000_000_000_000));
}

#[test]
fn test_stress_parse_size_extreme_and_boundary_bytes() {
    // 0 variations
    assert_eq!(parse_size_to_bytes("0"), Some(0));
    assert_eq!(parse_size_to_bytes("0 B"), Some(0));
    assert_eq!(parse_size_to_bytes("0.00 B"), Some(0));
    assert_eq!(parse_size_to_bytes("0.0000 KiB"), Some(0));
    assert_eq!(parse_size_to_bytes("0.0 MB"), Some(0));
    assert_eq!(parse_size_to_bytes("-0.0 B"), Some(0));

    // Decimal fractions rounding
    assert_eq!(parse_size_to_bytes("0.5 B"), Some(1)); // 0.5.round() -> 1
    assert_eq!(parse_size_to_bytes("0.4 B"), Some(0));
    assert_eq!(parse_size_to_bytes("1.5 KiB"), Some(1536)); // 1.5 * 1024 = 1536
    assert_eq!(parse_size_to_bytes("2.25 MiB"), Some((2.25 * 1048576.0) as u64));

    // Raw integers up to u64::MAX
    assert_eq!(parse_size_to_bytes("1"), Some(1));
    assert_eq!(parse_size_to_bytes("18446744073709551615"), Some(u64::MAX));
    assert_eq!(parse_size_to_bytes("18446744073709551614"), Some(u64::MAX - 1));

    // Large sizes with units
    let tib_16 = 16.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0;
    assert_eq!(parse_size_to_bytes("16 TiB"), Some(tib_16 as u64));

    // Negative numbers must return None
    assert_eq!(parse_size_to_bytes("-1"), None);
    assert_eq!(parse_size_to_bytes("-1 B"), None);
    assert_eq!(parse_size_to_bytes("-0.01 KiB"), None);
    assert_eq!(parse_size_to_bytes("-500 MiB"), None);
    assert_eq!(parse_size_to_bytes("-999999999999999999999999999 TiB"), None);

    // Overflow / Infinity / NaN
    assert_eq!(parse_size_to_bytes("NaN"), None);
    assert_eq!(parse_size_to_bytes("NaN B"), None);
    assert_eq!(parse_size_to_bytes("inf"), None);
    assert_eq!(parse_size_to_bytes("-inf"), None);
    assert_eq!(parse_size_to_bytes("+inf"), None);
    assert_eq!(parse_size_to_bytes("inf GiB"), None);
    assert_eq!(parse_size_to_bytes("1e30 TiB"), None); // massive overflow
}

#[test]
fn test_stress_parse_size_unusual_whitespace_and_nbsp() {
    // Unicode Non-breaking space (U+00A0)
    assert_eq!(parse_size_to_bytes("427.9\u{00A0}MB"), Some(427900000));
    assert_eq!(parse_size_to_bytes("15.5\u{00a0}MiB"), Some((15.5 * 1048576.0) as u64));
    assert_eq!(parse_size_to_bytes("\u{00A0}10\u{00A0}KiB\u{00A0}"), Some(10240));
    assert_eq!(parse_size_to_bytes("10\u{00A0}\u{00A0}KiB"), Some(10240));

    // Tab characters
    assert_eq!(parse_size_to_bytes("\t100\tMiB\t"), Some(104857600));
    assert_eq!(parse_size_to_bytes("100\t\t\tMiB"), Some(104857600));

    // Newlines and carriage returns
    assert_eq!(parse_size_to_bytes("\n200\r\nGiB\n"), Some(200 * 1024 * 1024 * 1024));

    // Mixed spaces, tabs, NBSP
    assert_eq!(parse_size_to_bytes("  50 \t \u{00A0} kB  "), Some(50000));

    // Empty / whitespace / NBSP only
    assert_eq!(parse_size_to_bytes(""), None);
    assert_eq!(parse_size_to_bytes("   "), None);
    assert_eq!(parse_size_to_bytes("\t\r\n"), None);
    assert_eq!(parse_size_to_bytes("\u{00A0}"), None);
    assert_eq!(parse_size_to_bytes("  \u{00A0}  \u{00A0}  "), None);
}

#[test]
fn test_stress_parse_size_invalid_formats() {
    assert_eq!(parse_size_to_bytes("not_a_number"), None);
    assert_eq!(parse_size_to_bytes("MiB"), None);
    assert_eq!(parse_size_to_bytes("12.34.56 MiB"), None);
    assert_eq!(parse_size_to_bytes("1,000 MiB"), None);
    assert_eq!(parse_size_to_bytes("10 UnknownUnit"), None);
    assert_eq!(parse_size_to_bytes("10 PB"), None); // Petabytes not supported
    assert_eq!(parse_size_to_bytes("10 PiB"), None);
    assert_eq!(parse_size_to_bytes("10 bits"), None);
    assert_eq!(parse_size_to_bytes("10 MiB extra token"), None);
    assert_eq!(parse_size_to_bytes("10 20 MiB"), None);
    assert_eq!(parse_size_to_bytes("++10 MiB"), None);
    assert_eq!(parse_size_to_bytes("10/20 MiB"), None);
}

// =============================================================================
// CHALLENGE SUITE 2: SortMode::SizeDesc Combinations
// =============================================================================

#[test]
fn test_stress_sort_mode_size_desc_all_combinations() {
    let mut packages = vec![
        UnifiedPackage::new("pkg-none-z", "1.0", PackageSource::Pacman),
        UnifiedPackage::new("pkg-zero-b", "1.0", PackageSource::Aur).with_size(0),
        UnifiedPackage::new("pkg-some-100", "1.0", PackageSource::Cargo).with_size(100),
        UnifiedPackage::new("pkg-none-a", "2.0", PackageSource::Flatpak),
        UnifiedPackage::new("pkg-zero-a", "1.0", PackageSource::Uv).with_size(0),
        UnifiedPackage::new("pkg-some-1000", "1.0", PackageSource::Pip).with_size(1000),
        UnifiedPackage::new("pkg-some-500", "1.0", PackageSource::Pacman).with_size(500),
        UnifiedPackage::new("pkg-none-a", "1.0", PackageSource::Flatpak), // same name as none-a 2.0
        UnifiedPackage::new("pkg-some-100-tie", "1.0", PackageSource::Cargo).with_size(100),
    ];

    SortMode::SizeDesc.sort_packages(&mut packages);

    let names_and_sizes: Vec<(&str, Option<u64>)> = packages
        .iter()
        .map(|p| (p.name.as_str(), p.size_bytes))
        .collect();

    // Verify ordering:
    // 1. 1000 bytes (pkg-some-1000)
    // 2. 500 bytes (pkg-some-500)
    // 3. 100 bytes (pkg-some-100)
    // 4. 100 bytes (pkg-some-100-tie)
    // 5. 0 bytes (pkg-zero-a)
    // 6. 0 bytes (pkg-zero-b)
    // 7. None (pkg-none-a v1.0)
    // 8. None (pkg-none-a v2.0)
    // 9. None (pkg-none-z)

    assert_eq!(names_and_sizes[0], ("pkg-some-1000", Some(1000)));
    assert_eq!(names_and_sizes[1], ("pkg-some-500", Some(500)));
    assert_eq!(names_and_sizes[2], ("pkg-some-100", Some(100)));
    assert_eq!(names_and_sizes[3], ("pkg-some-100-tie", Some(100)));
    assert_eq!(names_and_sizes[4], ("pkg-zero-a", Some(0)));
    assert_eq!(names_and_sizes[5], ("pkg-zero-b", Some(0)));
    assert_eq!(names_and_sizes[6], ("pkg-none-a", None));
    assert_eq!(packages[6].version, "1.0"); // version tiebreaker
    assert_eq!(names_and_sizes[7], ("pkg-none-a", None));
    assert_eq!(packages[7].version, "2.0");
    assert_eq!(names_and_sizes[8], ("pkg-none-z", None));

    // Core rule: all Some(0) must precede all None
    let first_none_index = packages.iter().position(|p| p.size_bytes.is_none()).unwrap();
    for (i, p) in packages.iter().enumerate() {
        if i < first_none_index {
            assert!(p.size_bytes.is_some(), "Package before first None must have Some size");
        } else {
            assert!(p.size_bytes.is_none(), "Package after first None must have None size");
        }
    }
}

#[test]
fn test_stress_sort_mode_large_dataset_invariant() {
    let mut packages = Vec::new();

    // Generate 300 packages across varied sizes: Some(0..50), Some(0), None
    for i in 0..300 {
        let name = format!("pkg-{:03}", 300 - i);
        let size = match i % 5 {
            0 => None,
            1 => Some(0),
            2 => Some((i * 1024) as u64),
            3 => Some(((300 - i) * 512) as u64),
            _ => Some(1000),
        };
        packages.push(
            UnifiedPackage::new(name, format!("{}.0", i % 10), PackageSource::Pacman)
                .with_size(size.unwrap_or(0)),
        );
        if size.is_none() {
            packages.last_mut().unwrap().size_bytes = None;
        }
    }

    SortMode::SizeDesc.sort_packages(&mut packages);

    // Verify invariant on entire list:
    let mut seen_none = false;
    let mut prev_size: Option<u64> = None;

    for (idx, p) in packages.iter().enumerate() {
        match p.size_bytes {
            Some(s) => {
                assert!(!seen_none, "Invariant violated: found Some({}) at index {} after None", s, idx);
                if let Some(prev) = prev_size {
                    assert!(
                        prev >= s,
                        "Invariant violated: sizes not descending: prev={}, current={}",
                        prev,
                        s
                    );
                    if prev == s {
                        let prev_p = &packages[idx - 1];
                        assert!(
                            prev_p.name.to_lowercase() <= p.name.to_lowercase(),
                            "Invariant violated: tiebreak by name failed: {} > {}",
                            prev_p.name,
                            p.name
                        );
                    }
                }
                prev_size = Some(s);
            }
            None => {
                seen_none = true;
                if idx > 0 && packages[idx - 1].size_bytes.is_none() {
                    let prev_p = &packages[idx - 1];
                    assert!(
                        prev_p.name.to_lowercase() <= p.name.to_lowercase(),
                        "Invariant violated: None tiebreak by name failed: {} > {}",
                        prev_p.name,
                        p.name
                    );
                }
            }
        }
    }
}

#[test]
fn test_stress_sort_mode_empty_and_single() {
    let mut empty: Vec<UnifiedPackage> = vec![];
    SortMode::SizeDesc.sort_packages(&mut empty);
    assert!(empty.is_empty());

    let mut single = vec![UnifiedPackage::new("alone", "1.0", PackageSource::Cargo)];
    SortMode::SizeDesc.sort_packages(&mut single);
    assert_eq!(single.len(), 1);
    assert_eq!(single[0].name, "alone");
}

#[test]
fn test_stress_sort_mode_case_insensitivity_tiebreak() {
    let mut packages = vec![
        UnifiedPackage::new("Zebra", "1.0", PackageSource::Pacman).with_size(100),
        UnifiedPackage::new("alpha", "1.0", PackageSource::Pacman).with_size(100),
        UnifiedPackage::new("Beta", "1.0", PackageSource::Pacman).with_size(100),
        UnifiedPackage::new("apple", "1.0", PackageSource::Pacman).with_size(100),
    ];

    SortMode::SizeDesc.sort_packages(&mut packages);
    assert_eq!(packages[0].name, "alpha");
    assert_eq!(packages[1].name, "apple");
    assert_eq!(packages[2].name, "Beta");
    assert_eq!(packages[3].name, "Zebra");
}

// =============================================================================
// CHALLENGE SUITE 3: UnifiedPackage Serialization Roundtrips
// =============================================================================

#[test]
fn test_stress_unified_package_serialization_all_sources() {
    let sources = [
        PackageSource::Pacman,
        PackageSource::Aur,
        PackageSource::Flatpak,
        PackageSource::Cargo,
        PackageSource::Uv,
        PackageSource::Pip,
    ];

    for src in sources {
        let pkg = UnifiedPackage::new(format!("pkg-{}", src.name()), "1.2.3-rc1", src)
            .with_size(1024 * 1024)
            .with_description(format!("Package from {}", src.display_name()))
            .with_dependencies(vec!["dep1".into(), "dep2>=2.0".into()])
            .with_files(vec!["/usr/bin/foo".into(), "/usr/share/doc/foo".into()])
            .with_binaries(vec!["foo".into(), "foo-cli".into()])
            .with_url("https://example.org/project")
            .with_license("MIT OR Apache-2.0")
            .with_install_date("2026-10-10 23:00:00");

        let json = serde_json::to_string(&pkg).expect("serialize to json");
        let deserialized: UnifiedPackage = serde_json::from_str(&json).expect("deserialize from json");

        assert_eq!(pkg, deserialized);
        assert_eq!(deserialized.source, src);
        assert_eq!(deserialized.size_bytes, Some(1024 * 1024));
    }
}

#[test]
fn test_stress_unified_package_serialization_zero_and_none_size() {
    // Some(0) must NOT be skipped or turned into None
    let pkg_zero = UnifiedPackage::new("meta-pkg", "1.0", PackageSource::Pacman).with_size(0);
    let json_zero = serde_json::to_string(&pkg_zero).expect("serialize");
    assert!(
        json_zero.contains("\"size_bytes\":0"),
        "Serialized JSON must contain size_bytes:0, got: {}",
        json_zero
    );
    let deser_zero: UnifiedPackage = serde_json::from_str(&json_zero).expect("deserialize");
    assert_eq!(deser_zero.size_bytes, Some(0));

    // None size must omit size_bytes
    let pkg_none = UnifiedPackage::new("unknown-size-pkg", "1.0", PackageSource::Flatpak);
    let json_none = serde_json::to_string(&pkg_none).expect("serialize");
    assert!(
        !json_none.contains("size_bytes"),
        "Serialized JSON must omit size_bytes when None, got: {}",
        json_none
    );
    let deser_none: UnifiedPackage = serde_json::from_str(&json_none).expect("deserialize");
    assert_eq!(deser_none.size_bytes, None);
}

#[test]
fn test_stress_unified_package_serialization_extreme_values_and_unicode() {
    let pkg = UnifiedPackage::new("unicode-📦", "99.99.99-α", PackageSource::Cargo)
        .with_size(u64::MAX)
        .with_description("Multi-line description\nWith \"quotes\", tabs\tand emoji: 🚀🔥\u{00A0}non-breaking")
        .with_dependencies(vec!["libstdc++".into(), "glibc>=2.34".into()])
        .with_files(vec!["/path with spaces/file.txt".into()])
        .with_url("https://example.com/?query=1&test=true#frag");

    let json = serde_json::to_string_pretty(&pkg).expect("serialize pretty");
    let deserialized: UnifiedPackage = serde_json::from_str(&json).expect("deserialize pretty");

    assert_eq!(pkg, deserialized);
    assert_eq!(deserialized.size_bytes, Some(u64::MAX));
    assert_eq!(deserialized.description.as_deref().unwrap(), pkg.description.as_deref().unwrap());
}

#[test]
fn test_stress_unified_package_deserialization_from_raw_json() {
    // Raw JSON with explicit nulls and missing optional fields
    let raw_json = r#"{
        "name": "minimal-pkg",
        "version": "0.1.0",
        "source": "pip",
        "size_bytes": null,
        "description": null
    }"#;

    let pkg: UnifiedPackage = serde_json::from_str(raw_json).expect("deserialize raw json");
    assert_eq!(pkg.name, "minimal-pkg");
    assert_eq!(pkg.version, "0.1.0");
    assert_eq!(pkg.source, PackageSource::Pip);
    assert_eq!(pkg.size_bytes, None);
    assert_eq!(pkg.description, None);
    assert!(pkg.dependencies.is_empty());
    assert!(pkg.files.is_empty());
    assert!(pkg.binaries.is_empty());
}

// =============================================================================
// CHALLENGE SUITE 4: format_bytes and format_bytes_compact
// =============================================================================

#[test]
fn test_stress_format_bytes() {
    assert_eq!(format_bytes(None), "Unknown");
    assert_eq!(format_bytes(Some(0)), "0 B");
    assert_eq!(format_bytes(Some(1)), "1 B");
    assert_eq!(format_bytes(Some(1023)), "1023 B");
    assert_eq!(format_bytes(Some(1024)), "1.00 KiB");
    assert_eq!(format_bytes(Some(1024 * 1024)), "1.00 MiB");
    assert_eq!(format_bytes(Some(1024 * 1024 * 1024)), "1.00 GiB");
    assert_eq!(format_bytes(Some(1024 * 1024 * 1024 * 1024)), "1.00 TiB");
    assert_eq!(format_bytes(Some(10 * 1024 * 1024 * 1024 * 1024)), "10.00 TiB");
}

#[test]
fn test_stress_format_bytes_compact() {
    assert_eq!(format_bytes_compact(None), "-");
    assert_eq!(format_bytes_compact(Some(0)), "0B");
    assert_eq!(format_bytes_compact(Some(999)), "999B");
    assert_eq!(format_bytes_compact(Some(1024)), "1.0K");
    assert_eq!(format_bytes_compact(Some(1024 * 1024)), "1.0M");
    assert_eq!(format_bytes_compact(Some(1024 * 1024 * 1024)), "1.0G");
    assert_eq!(format_bytes_compact(Some(1024 * 1024 * 1024 * 1024)), "1.0T");
}

#[test]
fn test_empirical_probe_format_bytes_compact_length_overflow() {
    // Contract check: docstring says "compact string (≤ 5 chars) for table rendering"
    // Does 500 MB (476.837 MiB) produce <= 5 characters?
    let res = format_bytes_compact(Some(500_000_000));
    println!("format_bytes_compact(Some(500_000_000)) = '{}' (len={})", res, res.len());
    // Record whether it violates <= 5 chars
    let exceeds_5 = res.len() > 5;
    assert!(exceeds_5, "Empirically proved that 500_000_000 produces '476.8M' with len 6 > 5");
}

#[test]
fn test_empirical_probe_parse_size_u64_max_plus_one() {
    // u64::MAX is 18446744073709551615
    // 18446744073709551616 is u64::MAX + 1
    let parsed_overflow_raw = parse_size_to_bytes("18446744073709551616");
    println!("parse_size_to_bytes('18446744073709551616') = {:?}", parsed_overflow_raw);

    let parsed_overflow_unit = parse_size_to_bytes("18446744073709551616 B");
    println!("parse_size_to_bytes('18446744073709551616 B') = {:?}", parsed_overflow_unit);

    let parsed_max = parse_size_to_bytes("18446744073709551615");
    assert_eq!(parsed_max, Some(u64::MAX));

    // Note: Due to f64 precision rounding (2^64 == u64::MAX as f64 == 18446744073709551616.0),
    // 18446744073709551616 B evaluates total <= u64::MAX as f64, saturating to u64::MAX.
}

#[test]
fn test_empirical_probe_parse_size_no_space_and_special_spaces() {
    // What happens if raw string has no space between number and unit: "100MiB"?
    let no_space = parse_size_to_bytes("100MiB");
    println!("parse_size_to_bytes('100MiB') = {:?}", no_space);
    assert_eq!(no_space, None, "Expected None for unspaced units like '100MiB'");

    // Narrow non-breaking space U+202F
    let narrow_nbsp = parse_size_to_bytes("427.9\u{202F}MB");
    println!("parse_size_to_bytes('427.9\\u{{202F}}MB') = {:?}", narrow_nbsp);

    // Thin space U+2009
    let thin_space = parse_size_to_bytes("427.9\u{2009}MB");
    println!("parse_size_to_bytes('427.9\\u{{2009}}MB') = {:?}", thin_space);
}

#[test]
fn test_empirical_probe_sort_mode_tiebreakers() {
    // Packages with identical sizes and names, differing only by version
    let mut pkgs = vec![
        UnifiedPackage::new("rustc", "1.82.0", PackageSource::Cargo).with_size(500),
        UnifiedPackage::new("rustc", "1.80.0", PackageSource::Cargo).with_size(500),
        UnifiedPackage::new("rustc", "1.85.0", PackageSource::Cargo).with_size(500),
    ];
    SortMode::SizeDesc.sort_packages(&mut pkgs);
    assert_eq!(pkgs[0].version, "1.80.0");
    assert_eq!(pkgs[1].version, "1.82.0");
    assert_eq!(pkgs[2].version, "1.85.0");

    // Packages with identical sizes, names, versions, but differing source
    let mut pkgs_src = vec![
        UnifiedPackage::new("curl", "8.0", PackageSource::Pacman).with_size(100),
        UnifiedPackage::new("curl", "8.0", PackageSource::Cargo).with_size(100),
    ];
    // SizeDesc sort is stable and doesn't crash
    SortMode::SizeDesc.sort_packages(&mut pkgs_src);
    assert_eq!(pkgs_src.len(), 2);
}

#[test]
fn test_empirical_probe_unified_package_null_deserialization() {
    // Check if serde handles explicit null for dependencies, files, binaries
    let null_deps_json = r#"{
        "name": "test-pkg",
        "version": "1.0",
        "source": "pacman",
        "dependencies": null
    }"#;
    let res: Result<UnifiedPackage, _> = serde_json::from_str(null_deps_json);
    println!("Deserialization with 'dependencies: null' result: {:?}", res);
    // Serde requires `Vec<String>` to be an array, null is rejected by default unless custom deserializer is used.
    assert!(res.is_err(), "Standard serde Vec rejects explicit null without custom deserializer");
}
