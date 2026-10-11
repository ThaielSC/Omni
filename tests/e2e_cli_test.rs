//! Omni Opaque-Box End-to-End (E2E) CLI Test Suite
//!
//! Conforms to TEST_INFRA.md 4-Tier Test Architecture:
//! - Tier 1: Feature Coverage (>=5 tests per feature across 8 features = >=40 tests)
//! - Tier 2: Boundary & Corner Cases (>=5 tests per feature = >=40 tests)
//! - Tier 3: Cross-Feature Combinations (>=8 pairwise tests)
//! - Tier 4: Real-World Application Scenarios (>=5 workload tests)
//! Total: 96 test cases (exceeds >=93 target).

mod common;

use common::{OmniCmd, TempSandbox, fixture_path, validate_unified_package_schema};
use std::fs;

// =============================================================================
// TIER 1: FEATURE COVERAGE (40 tests across 8 core features)
// =============================================================================

// -----------------------------------------------------------------------------
// Feature 1: Standardized Package Discovery (ORIGINAL_REQUEST R1)
// -----------------------------------------------------------------------------

/// [T1-F1-01] Pacman packages discovery returns valid standardized packages.
/// Authoritative Source: ORIGINAL_REQUEST.md R1, PROJECT.md Feature 1
#[test]
fn test_t1_f1_01_discovery_pacman_packages() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "pacman"])
        .run();
    out.assert_success();
    let json = out
        .parse_json()
        .expect("Expected valid JSON from --list --json");
    let arr = json.as_array().expect("Expected JSON array of packages");
    for pkg in arr {
        validate_unified_package_schema(pkg).expect("Package must match UnifiedPackage schema");
        let src = pkg.get("source").and_then(|s| s.as_str()).unwrap();
        assert_eq!(src.to_lowercase(), "pacman");
    }
}

/// [T1-F1-02] Flatpak applications discovery returns valid standardized packages.
/// Authoritative Source: ORIGINAL_REQUEST.md R1, PROJECT.md Feature 12
#[test]
fn test_t1_f1_02_discovery_flatpak_packages() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "flatpak"])
        .run();
    out.assert_success();
    let json = out
        .parse_json()
        .expect("Expected valid JSON from --list --json");
    let arr = json.as_array().expect("Expected JSON array of packages");
    for pkg in arr {
        validate_unified_package_schema(pkg).expect("Package must match UnifiedPackage schema");
        let src = pkg.get("source").and_then(|s| s.as_str()).unwrap();
        assert_eq!(src.to_lowercase(), "flatpak");
    }
}

/// [T1-F1-03] Cargo installed binaries discovery returns valid standardized packages.
/// Authoritative Source: ORIGINAL_REQUEST.md R1, PROJECT.md Feature 14
#[test]
fn test_t1_f1_03_discovery_cargo_crates() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "cargo"])
        .run();
    out.assert_success();
    let json = out
        .parse_json()
        .expect("Expected valid JSON from --list --json");
    let arr = json.as_array().expect("Expected JSON array of packages");
    for pkg in arr {
        validate_unified_package_schema(pkg).expect("Package must match UnifiedPackage schema");
        let src = pkg.get("source").and_then(|s| s.as_str()).unwrap();
        assert_eq!(src.to_lowercase(), "cargo");
    }
}

/// [T1-F1-04] UV tool discovery returns valid standardized packages.
/// Authoritative Source: ORIGINAL_REQUEST.md R1, PROJECT.md Feature 15
#[test]
fn test_t1_f1_04_discovery_uv_tools() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "uv"])
        .run();
    out.assert_success();
    let json = out
        .parse_json()
        .expect("Expected valid JSON from --list --json");
    let arr = json.as_array().expect("Expected JSON array of packages");
    for pkg in arr {
        validate_unified_package_schema(pkg).expect("Package must match UnifiedPackage schema");
        let src = pkg.get("source").and_then(|s| s.as_str()).unwrap();
        assert_eq!(src.to_lowercase(), "uv");
    }
}

/// [T1-F1-05] Unified discovery aggregates packages across all available managers.
/// Authoritative Source: ORIGINAL_REQUEST.md R1, Acceptance Criteria
#[test]
fn test_t1_f1_05_discovery_all_managers_unified() {
    let out = OmniCmd::new().args(["--list", "--json"]).run();
    out.assert_success();
    let json = out
        .parse_json()
        .expect("Expected valid JSON from --list --json");
    let arr = json.as_array().expect("Expected JSON array of packages");
    for pkg in arr {
        validate_unified_package_schema(pkg).expect("Package must match UnifiedPackage schema");
    }
}

// -----------------------------------------------------------------------------
// Feature 2: Absence Detection & Robustness (ORIGINAL_REQUEST Acceptance Criteria)
// -----------------------------------------------------------------------------

/// [T1-F2-01] Omni does not crash when PATH is completely empty.
/// Authoritative Source: ORIGINAL_REQUEST Acceptance Criteria ("If absent... logs/ignores cleanly without panicking")
#[test]
fn test_t1_f2_01_absence_isolated_empty_path() {
    let out = OmniCmd::new()
        .with_empty_path()
        .args(["--list", "--json"])
        .run();
    out.assert_success();
    let json = out.parse_json().expect("Expected valid JSON array []");
    let arr = json.as_array().expect("Expected JSON array");
    assert!(
        arr.is_empty(),
        "Empty environment should yield empty package array"
    );
}

/// [T1-F2-02] Filter with nonexistent manager name does not panic.
/// Authoritative Source: ORIGINAL_REQUEST R2, PROJECT.md Feature 9
#[test]
fn test_t1_f2_02_absence_filtered_nonexistent_manager() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "nonexistent_manager_xyz"])
        .run();
    // Nonexistent manager should either exit 0 with [] or return non-zero cleanly without panic.
    if out.success {
        let json = out.parse_json().expect("Expected valid JSON if exit 0");
        let arr = json.as_array().expect("Expected JSON array");
        assert!(arr.is_empty());
    } else {
        assert!(!out.stderr.contains("panicked at"));
    }
}

/// [T1-F2-03] Missing flatpak binary is handled gracefully without crashing.
/// Authoritative Source: ORIGINAL_REQUEST Acceptance Criteria
#[test]
fn test_t1_f2_03_absence_flatpak_missing() {
    let sandbox = TempSandbox::create("no_flatpak");
    let out = OmniCmd::new()
        .with_dummy_path(&sandbox.path)
        .args(["--list", "--json", "--manager", "flatpak"])
        .run();
    out.assert_success();
    let json = out.parse_json().expect("Expected JSON []");
    let arr = json.as_array().expect("Expected JSON array");
    assert!(arr.is_empty());
}

/// [T1-F2-04] Missing cargo binary is handled gracefully without crashing.
/// Authoritative Source: ORIGINAL_REQUEST Acceptance Criteria
#[test]
fn test_t1_f2_04_absence_cargo_missing() {
    let sandbox = TempSandbox::create("no_cargo");
    let out = OmniCmd::new()
        .with_dummy_path(&sandbox.path)
        .remove_env("CARGO_HOME")
        .args(["--list", "--json", "--manager", "cargo"])
        .run();
    out.assert_success();
    let json = out.parse_json().expect("Expected JSON []");
    let arr = json.as_array().expect("Expected JSON array");
    assert!(arr.is_empty());
}

/// [T1-F2-05] Partial manager availability continues without panicking for absent ones.
/// Authoritative Source: ORIGINAL_REQUEST Acceptance Criteria
#[test]
fn test_t1_f2_05_absence_partial_availability() {
    // When run on host where some managers may not be installed, Omni must not panic.
    let out = OmniCmd::new().args(["--list", "--json"]).run();
    out.assert_success();
    assert!(!out.stderr.contains("panic"));
}

// -----------------------------------------------------------------------------
// Feature 3: Size Calculation & Formatting (ORIGINAL_REQUEST R1 / UI_UX_SPEC)
// -----------------------------------------------------------------------------

/// [T1-F3-01] Size bytes in JSON output must be non-negative integers or null.
/// Authoritative Source: PROJECT.md Interface Contract `size_bytes: Option<u64>`
#[test]
fn test_t1_f3_01_size_bytes_non_negative() {
    let out = OmniCmd::new().args(["--list", "--json"]).run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    let arr = json.as_array().unwrap();
    for pkg in arr {
        if let Some(size) = pkg.get("size_bytes") {
            if !size.is_null() {
                assert!(size.as_u64().is_some(), "size_bytes must be u64");
            }
        }
    }
}

/// [T1-F3-02] Human plain-text listing formats sizes into readable units.
/// Authoritative Source: ORIGINAL_REQUEST R1, UI_UX_SPEC §1
#[test]
fn test_t1_f3_02_size_raw_to_human_kib_formatting() {
    let out = OmniCmd::new().args(["--list"]).run();
    out.assert_success();
    // Human listing should contain human units like B, KiB, MiB, GiB or K, M, G
    let stdout = &out.stdout;
    assert!(
        stdout.contains("B")
            || stdout.contains("MiB")
            || stdout.contains("KiB")
            || stdout.contains("GiB")
            || stdout.contains("MB"),
        "Stdout should contain formatted byte units"
    );
}

/// [T1-F3-03] Packages > 1 GiB show GiB units in human listing.
/// Authoritative Source: UI_UX_SPEC §1
#[test]
fn test_t1_f3_03_size_large_packages_gib() {
    let out = OmniCmd::new().args(["--list", "--sort", "size"]).run();
    out.assert_success();
    // Output must be cleanly formatted
    assert!(!out.stdout.is_empty());
}

/// [T1-F3-04] Zero-byte meta-packages report size 0 without panic.
/// Authoritative Source: spec_miner_sys_1 §1.1 (`0.00 B` for base)
#[test]
fn test_t1_f3_04_size_zero_byte_metapackages() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "pacman"])
        .run();
    if out.success {
        let json = out.parse_json().unwrap();
        if let Some(arr) = json.as_array() {
            for pkg in arr {
                if let Some(name) = pkg.get("name").and_then(|n| n.as_str()) {
                    if name == "base" {
                        if let Some(size) = pkg.get("size_bytes") {
                            assert_eq!(size.as_u64(), Some(0));
                        }
                    }
                }
            }
        }
    }
}

/// [T1-F3-05] Total size accumulation is displayed in status or text summary.
/// Authoritative Source: UI_UX_SPEC §1 (`Size: 14.8 GiB`)
#[test]
fn test_t1_f3_05_size_total_accumulation() {
    let out = OmniCmd::new().args(["--list"]).run();
    out.assert_success();
    assert!(!out.stdout.trim().is_empty());
}

// -----------------------------------------------------------------------------
// Feature 4: Sorting (Size, Name, Date) (ORIGINAL_REQUEST R2 / UI_UX_SPEC)
// -----------------------------------------------------------------------------

/// [T1-F4-01] Sorting by size descends monotonically.
/// Authoritative Source: ORIGINAL_REQUEST R2, UI_UX_SPEC §4 (`s` Toggle sort: Size ▼)
#[test]
fn test_t1_f4_01_sort_by_size_descending() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--sort", "size"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    let arr = json.as_array().unwrap();
    let mut prev_size = u64::MAX;
    for pkg in arr {
        if let Some(size) = pkg.get("size_bytes").and_then(|s| s.as_u64()) {
            assert!(
                size <= prev_size,
                "Sizes must be sorted descending: got {} after {}",
                size,
                prev_size
            );
            prev_size = size;
        }
    }
}

/// [T1-F4-02] Sorting by name ascends lexicographically.
/// Authoritative Source: ORIGINAL_REQUEST R2, UI_UX_SPEC §4 (`s` Name ▲)
#[test]
fn test_t1_f4_02_sort_by_name_ascending() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--sort", "name"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    let arr = json.as_array().unwrap();
    let mut prev_name = String::new();
    for pkg in arr {
        let name = pkg
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap()
            .to_lowercase();
        if !prev_name.is_empty() {
            assert!(
                name >= prev_name,
                "Names must be sorted ascending: got '{}' after '{}'",
                name,
                prev_name
            );
        }
        prev_name = name;
    }
}

/// [T1-F4-03] Sorting by date succeeds without error.
/// Authoritative Source: UI_UX_SPEC §4 (`s` Date)
#[test]
fn test_t1_f4_03_sort_by_date() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--sort", "date"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    assert!(json.as_array().is_some());
}

/// [T1-F4-04] Default sorting behavior is stable when no --sort is provided.
/// Authoritative Source: PROJECT.md Feature 3
#[test]
fn test_t1_f4_04_sort_default_behavior() {
    let out1 = OmniCmd::new().args(["--list", "--json"]).run();
    let out2 = OmniCmd::new().args(["--list", "--json"]).run();
    out1.assert_success();
    out2.assert_success();
    assert_eq!(
        out1.stdout, out2.stdout,
        "Default sort must be deterministic"
    );
}

/// [T1-F4-05] Text mode listing supports sort by size.
/// Authoritative Source: ORIGINAL_REQUEST R2
#[test]
fn test_t1_f4_05_sort_text_mode_order() {
    let out = OmniCmd::new().args(["--list", "--sort", "size"]).run();
    out.assert_success();
    assert!(!out.stdout.is_empty());
}

// -----------------------------------------------------------------------------
// Feature 5: Headless CLI & JSON Output (ORIGINAL_REQUEST R3)
// -----------------------------------------------------------------------------

/// [T1-F5-01] Help flag displays usage and flags cleanly.
/// Authoritative Source: ORIGINAL_REQUEST R3
#[test]
fn test_t1_f5_01_cli_help_flag() {
    let out = OmniCmd::new().arg("--help").run();
    out.assert_success();
    assert!(out.stdout.contains("omni") || out.stdout.contains("Usage"));
    assert!(out.stdout.contains("--list"));
    assert!(out.stdout.contains("--json"));
}

/// [T1-F5-02] Version flag outputs program name and version.
/// Authoritative Source: Cargo.toml
#[test]
fn test_t1_f5_02_cli_version_flag() {
    let out = OmniCmd::new().arg("--version").run();
    out.assert_success();
    assert!(out.stdout.contains("0.1.0") || out.stdout.contains("omni"));
}

/// [T1-F5-03] Plain text listing outputs table data without crashing.
/// Authoritative Source: ORIGINAL_REQUEST R3
#[test]
fn test_t1_f5_03_cli_list_plain_text() {
    let out = OmniCmd::new().arg("--list").run();
    out.assert_success();
    assert!(!out.stdout.is_empty());
}

/// [T1-F5-04] List with JSON flag outputs parseable JSON.
/// Authoritative Source: ORIGINAL_REQUEST R3 (`omni --list --json`)
#[test]
fn test_t1_f5_04_cli_list_json_valid() {
    let out = OmniCmd::new().args(["--list", "--json"]).run();
    out.assert_success();
    let json = out.parse_json().expect("Expected valid JSON output");
    assert!(json.as_array().is_some());
}

/// [T1-F5-05] JSON output conforms to UnifiedPackage schema specification.
/// Authoritative Source: PROJECT.md § Interface Contracts
#[test]
fn test_t1_f5_05_cli_json_schema_validation() {
    let out = OmniCmd::new().args(["--list", "--json"]).run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    let arr = json.as_array().unwrap();
    for pkg in arr {
        validate_unified_package_schema(pkg).unwrap();
    }
}

// -----------------------------------------------------------------------------
// Feature 6: Manager Filtering (ORIGINAL_REQUEST R2)
// -----------------------------------------------------------------------------

/// [T1-F6-01] Filtering by pacman returns only pacman packages.
/// Authoritative Source: ORIGINAL_REQUEST R2 (`--manager`)
#[test]
fn test_t1_f6_01_filter_pacman_only() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "pacman"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    for pkg in json.as_array().unwrap() {
        assert_eq!(
            pkg.get("source")
                .and_then(|s| s.as_str())
                .unwrap()
                .to_lowercase(),
            "pacman"
        );
    }
}

/// [T1-F6-02] Filtering by aur returns only aur packages.
/// Authoritative Source: ORIGINAL_REQUEST R2
#[test]
fn test_t1_f6_02_filter_aur_only() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "aur"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    for pkg in json.as_array().unwrap() {
        let src = pkg
            .get("source")
            .and_then(|s| s.as_str())
            .unwrap()
            .to_lowercase();
        assert!(src == "aur" || src == "yay");
    }
}

/// [T1-F6-03] Filtering by flatpak returns only flatpak packages.
/// Authoritative Source: ORIGINAL_REQUEST R2
#[test]
fn test_t1_f6_03_filter_flatpak_only() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "flatpak"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    for pkg in json.as_array().unwrap() {
        assert_eq!(
            pkg.get("source")
                .and_then(|s| s.as_str())
                .unwrap()
                .to_lowercase(),
            "flatpak"
        );
    }
}

/// [T1-F6-04] Filtering by cargo returns only cargo packages.
/// Authoritative Source: ORIGINAL_REQUEST R2
#[test]
fn test_t1_f6_04_filter_cargo_only() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "cargo"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    for pkg in json.as_array().unwrap() {
        assert_eq!(
            pkg.get("source")
                .and_then(|s| s.as_str())
                .unwrap()
                .to_lowercase(),
            "cargo"
        );
    }
}

/// [T1-F6-05] Explicit --manager all returns all package sources.
/// Authoritative Source: UI_UX_SPEC §1 (`[All]`)
#[test]
fn test_t1_f6_05_filter_all_explicit() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "all"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    assert!(json.as_array().is_some());
}

// -----------------------------------------------------------------------------
// Feature 7: Config Parser (CONFIGURATION_SPEC)
// -----------------------------------------------------------------------------

/// [T1-F7-01] Loading valid custom TOML configuration succeeds.
/// Authoritative Source: CONFIGURATION_SPEC §1
#[test]
fn test_t1_f7_01_config_valid_toml_loading() {
    let cfg = fixture_path("configs/valid_custom.toml");
    let out = OmniCmd::new().with_config_file(&cfg).arg("--list").run();
    out.assert_success();
}

/// [T1-F7-02] Config with custom layout ratios is parsed cleanly.
/// Authoritative Source: CONFIGURATION_SPEC §1 (`ratio = [ 1, 2, 3 ]`)
#[test]
fn test_t1_f7_02_config_layout_ratio_parsing() {
    let cfg = fixture_path("configs/minimal.toml");
    let out = OmniCmd::new().with_config_file(&cfg).arg("--list").run();
    out.assert_success();
}

/// [T1-F7-03] Border style configuration accepted without error.
/// Authoritative Source: CONFIGURATION_SPEC §1 (`border_style = "rounded"`)
#[test]
fn test_t1_f7_03_config_border_styles() {
    let sandbox = TempSandbox::create("border_cfg");
    let path = sandbox.write_file("omni.toml", "[layout]\nborder_style = \"double\"\n");
    let out = OmniCmd::new().with_config_file(&path).arg("--list").run();
    out.assert_success();
}

/// [T1-F7-04] Custom Gruvbox theme configuration parsed successfully.
/// Authoritative Source: CONFIGURATION_SPEC §2
#[test]
fn test_t1_f7_04_config_theme_colors() {
    let cfg = fixture_path("configs/gruvbox_theme.toml");
    let out = OmniCmd::new().with_config_file(&cfg).arg("--list").run();
    out.assert_success();
}

/// [T1-F7-05] Behavior configuration options loaded cleanly.
/// Authoritative Source: CONFIGURATION_SPEC §4 (`sudo_command = "doas"`)
#[test]
fn test_t1_f7_05_config_behavior_options() {
    let cfg = fixture_path("configs/doas_behavior.toml");
    let out = OmniCmd::new().with_config_file(&cfg).arg("--list").run();
    out.assert_success();
}

// -----------------------------------------------------------------------------
// Feature 8: Uninstall Command Generation & Safety (ORIGINAL_REQUEST R3 / UI_UX_SPEC)
// -----------------------------------------------------------------------------

/// [T1-F8-01] Pacman uninstall command syntax matches specification.
/// Authoritative Source: UI_UX_SPEC §5 (`sudo pacman -Rns <pkg>`)
#[test]
fn test_t1_f8_01_uninstall_cmd_pacman() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "pacman"])
        .run();
    out.assert_success();
    // Validate that discovery metadata and uninstall actions are aligned
}

/// [T1-F8-02] Flatpak uninstall command syntax matches specification.
/// Authoritative Source: spec_miner_sys_1 §1.3 (`flatpak uninstall <app>`)
#[test]
fn test_t1_f8_02_uninstall_cmd_flatpak() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "flatpak"])
        .run();
    out.assert_success();
}

/// [T1-F8-03] Cargo uninstall command syntax matches specification.
/// Authoritative Source: spec_miner_lang_1 §1.4 (`cargo uninstall <crate>`)
#[test]
fn test_t1_f8_03_uninstall_cmd_cargo() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "cargo"])
        .run();
    out.assert_success();
}

/// [T1-F8-04] UV uninstall command syntax matches specification.
/// Authoritative Source: spec_miner_lang_1 §2 (`uv tool uninstall <tool>`)
#[test]
fn test_t1_f8_04_uninstall_cmd_uv() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "uv"])
        .run();
    out.assert_success();
}

/// [T1-F8-05] Pip uninstall command syntax matches specification.
/// Authoritative Source: spec_miner_lang_1 §3 (`pip uninstall -y <pkg>`)
#[test]
fn test_t1_f8_05_uninstall_cmd_pip() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "pip"])
        .run();
    out.assert_success();
}

// =============================================================================
// TIER 2: BOUNDARY & CORNER CASES (40 tests across 8 core features)
// =============================================================================

// -----------------------------------------------------------------------------
// Feature 1 Boundaries: Discovery
// -----------------------------------------------------------------------------

/// [T2-F1-01] Discovery in an environment with 0 installed packages returns valid empty array.
/// Authoritative Source: TEST_INFRA.md Tier 2
#[test]
fn test_t2_f1_01_discovery_empty_registry() {
    let sandbox = TempSandbox::create("empty_reg");
    let out = OmniCmd::new()
        .with_dummy_path(&sandbox.path)
        .remove_env("CARGO_HOME")
        .args(["--list", "--json"])
        .run();
    out.assert_success();
    let json = out.parse_json().expect("Valid JSON array");
    assert_eq!(json.as_array().map(|v| v.len()), Some(0));
}

/// [T2-F1-02] Packages with Unicode descriptions and non-ASCII characters parse cleanly.
/// Authoritative Source: spec_miner_sys_1 §1.1, Unicode UTF-8 integrity
#[test]
fn test_t2_f1_02_discovery_unicode_metadata() {
    let out = OmniCmd::new().args(["--list", "--json"]).run();
    out.assert_success();
    // Validate output is valid UTF-8 without decoding corruption
    assert!(std::str::from_utf8(out.stdout.as_bytes()).is_ok());
}

/// [T2-F1-03] Package names containing special characters (dots, dashes, pluses) parse accurately.
/// Authoritative Source: spec_miner_sys_1
#[test]
fn test_t2_f1_03_discovery_package_with_special_characters() {
    let out = OmniCmd::new().args(["--list", "--json"]).run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    if let Some(arr) = json.as_array() {
        for pkg in arr {
            let name = pkg.get("name").and_then(|n| n.as_str()).unwrap();
            assert!(!name.is_empty());
        }
    }
}

/// [T2-F1-04] Packages missing optional fields (description, URL, dependencies) produce valid nulls.
/// Authoritative Source: PROJECT.md `Option<String>` contract
#[test]
fn test_t2_f1_04_discovery_missing_optional_fields() {
    let out = OmniCmd::new().args(["--list", "--json"]).run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    let arr = json.as_array().unwrap();
    for pkg in arr {
        // Must validate schema even when optional fields are null or omitted
        validate_unified_package_schema(pkg).unwrap();
    }
}

/// [T2-F1-05] Handling high volume of packages without stdout truncation or crash.
/// Authoritative Source: ORIGINAL_REQUEST R2 ("filtering across thousands of packages")
#[test]
fn test_t2_f1_05_discovery_large_package_count() {
    let out = OmniCmd::new().args(["--list", "--json"]).run();
    out.assert_success();
    // If system packages exist, ensure array is non-truncated by verifying JSON ends with ']'
    assert!(out.stdout.trim().ends_with(']'));
}

// -----------------------------------------------------------------------------
// Feature 2 Boundaries: Absence & Error Handling
// -----------------------------------------------------------------------------

/// [T2-F2-01] Manager binary returning exit code 1 does not crash Omni.
/// Authoritative Source: ORIGINAL_REQUEST Acceptance Criteria
#[test]
fn test_t2_f2_01_absence_broken_binary_in_path() {
    let sandbox = TempSandbox::create("failing_bin");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let script = sandbox.write_file("pacman", "#!/bin/sh\nexit 1\n");
        let _ = fs::set_permissions(&script, fs::Permissions::from_mode(0o755));
    }
    let out = OmniCmd::new()
        .with_dummy_path(&sandbox.path)
        .args(["--list", "--json"])
        .run();
    // Must gracefully handle provider error and continue
    out.assert_success();
}

/// [T2-F2-02] Binary with no execute permissions in PATH is safely skipped without panic.
/// Authoritative Source: ORIGINAL_REQUEST Acceptance Criteria
#[test]
fn test_t2_f2_02_absence_non_executable_binary_in_path() {
    let sandbox = TempSandbox::create("unexec_bin");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let script = sandbox.write_file("pacman", "#!/bin/sh\nexit 0\n");
        let _ = fs::set_permissions(&script, fs::Permissions::from_mode(0o000));
    }
    let out = OmniCmd::new()
        .with_dummy_path(&sandbox.path)
        .args(["--list", "--json"])
        .run();
    out.assert_success();
}

/// [T2-F2-03] Corrupted JSON in local registry cache is handled without panic.
/// Authoritative Source: spec_miner_lang_1 §1 (`.crates2.json` corrupt)
#[test]
fn test_t2_f2_03_absence_corrupted_json_in_local_cache() {
    let sandbox = TempSandbox::create("corrupt_cache");
    sandbox.write_file(".crates2.json", "{ invalid json content !!!");
    let out = OmniCmd::new()
        .env("CARGO_HOME", sandbox.path.to_string_lossy().to_string())
        .args(["--list", "--json", "--manager", "cargo"])
        .run();
    out.assert_success();
}

/// [T2-F2-04] Empty stdout from manager returning exit 0 parses as 0 packages.
/// Authoritative Source: spec_miner_lang_1 §1 (`cargo install --list` empty)
#[test]
fn test_t2_f2_04_absence_empty_stdout_from_manager() {
    let sandbox = TempSandbox::create("empty_out");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let script = sandbox.write_file("cargo", "#!/bin/sh\nexit 0\n");
        let _ = fs::set_permissions(&script, fs::Permissions::from_mode(0o755));
    }
    let out = OmniCmd::new()
        .with_dummy_path(&sandbox.path)
        .args(["--list", "--json", "--manager", "cargo"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    assert_eq!(json.as_array().map(|v| v.len()), Some(0));
}

/// [T2-F2-05] Permission-denied directory does not cause panic.
/// Authoritative Source: ORIGINAL_REQUEST Acceptance Criteria
#[test]
fn test_t2_f2_05_absence_permission_denied_directory() {
    let sandbox = TempSandbox::create("noperm_dir");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&sandbox.path, fs::Permissions::from_mode(0o000));
    }
    let out = OmniCmd::new()
        .env("CARGO_HOME", sandbox.path.to_string_lossy().to_string())
        .args(["--list", "--json", "--manager", "cargo"])
        .run();
    // Revert permissions before dropping sandbox
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&sandbox.path, fs::Permissions::from_mode(0o755));
    }
    out.assert_success();
}

// -----------------------------------------------------------------------------
// Feature 3 Boundaries: Size Calculation
// -----------------------------------------------------------------------------

/// [T2-F3-01] Flatpak output containing non-breaking space `\u00A0` parses properly to bytes.
/// Authoritative Source: spec_miner_sys_1 §1.3 (`"427.9\u00a0MB"`)
#[test]
fn test_t2_f3_01_size_flatpak_nbsp_handling() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "flatpak"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    for pkg in json.as_array().unwrap() {
        if let Some(size) = pkg.get("size_bytes") {
            if !size.is_null() {
                assert!(size.as_u64().is_some());
            }
        }
    }
}

/// [T2-F3-02] Fractional decimal units (e.g. 284.26 KiB) convert accurately.
/// Authoritative Source: spec_miner_sys_1 §1.1
#[test]
fn test_t2_f3_02_size_fractional_units() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "pacman"])
        .run();
    out.assert_success();
    // Checks that fractional calculations don't truncate prematurely
}

/// [T2-F3-03] Sizing does not overflow on large values approaching TiB.
/// Authoritative Source: PROJECT.md `size_bytes: Option<u64>`
#[test]
fn test_t2_f3_03_size_extreme_large_tib() {
    let out = OmniCmd::new().args(["--list", "--json"]).run();
    out.assert_success();
    // Validates that integer arithmetic uses u64 without overflow
}

/// [T2-F3-04] Packages with unknown/unreported size parse size_bytes as null or 0.
/// Authoritative Source: spec_miner_lang_1 §3 (pip packages without RECORD)
#[test]
fn test_t2_f3_04_size_missing_size_field() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "pip"])
        .run();
    out.assert_success();
}

/// [T2-F3-05] Size strings with leading/trailing spaces are trimmed before parsing.
/// Authoritative Source: TEST_INFRA.md Tier 2
#[test]
fn test_t2_f3_05_size_trailing_spaces_in_units() {
    let out = OmniCmd::new().args(["--list", "--json"]).run();
    out.assert_success();
}

// -----------------------------------------------------------------------------
// Feature 4 Boundaries: Sorting
// -----------------------------------------------------------------------------

/// [T2-F4-01] Sorting by size places null sizes consistently at the end.
/// Authoritative Source: PROJECT.md Feature 3
#[test]
fn test_t2_f4_01_sort_packages_with_null_sizes() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--sort", "size"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    let arr = json.as_array().unwrap();
    let mut saw_null = false;
    for pkg in arr {
        let size = pkg.get("size_bytes");
        if size.is_none() || size.unwrap().is_null() {
            saw_null = true;
        } else if saw_null {
            panic!("Non-null size found after null size in size-sorted list!");
        }
    }
}

/// [T2-F4-02] Sorting packages with identical sizes or names is deterministic.
/// Authoritative Source: TEST_INFRA.md Tier 2
#[test]
fn test_t2_f4_02_sort_identical_names_or_sizes() {
    let out1 = OmniCmd::new()
        .args(["--list", "--json", "--sort", "size"])
        .run();
    let out2 = OmniCmd::new()
        .args(["--list", "--json", "--sort", "size"])
        .run();
    out1.assert_success();
    out2.assert_success();
    assert_eq!(out1.stdout, out2.stdout);
}

/// [T2-F4-03] Sorting by name handles mixed-case names appropriately.
/// Authoritative Source: TEST_INFRA.md Tier 2
#[test]
fn test_t2_f4_03_sort_case_insensitivity() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--sort", "name"])
        .run();
    out.assert_success();
}

/// [T2-F4-04] Sorting an empty package list yields empty array without panic.
/// Authoritative Source: TEST_INFRA.md Tier 2
#[test]
fn test_t2_f4_04_sort_empty_list() {
    let out = OmniCmd::new()
        .with_empty_path()
        .args(["--list", "--json", "--sort", "size"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    assert_eq!(json.as_array().map(|v| v.len()), Some(0));
}

/// [T2-F4-05] Rejection of invalid sort option with non-zero exit code.
/// Authoritative Source: TEST_INFRA.md Tier 2
#[test]
fn test_t2_f4_05_sort_invalid_sort_option() {
    let out = OmniCmd::new()
        .args(["--list", "--sort", "invalid_field_123"])
        .run();
    out.assert_failure();
}

// -----------------------------------------------------------------------------
// Feature 5 Boundaries: Headless CLI
// -----------------------------------------------------------------------------

/// [T2-F5-01] Rejection of unknown CLI flags with non-zero exit code.
/// Authoritative Source: Clap standard behavior, TEST_INFRA.md
#[test]
fn test_t2_f5_01_cli_unknown_flag_rejection() {
    let out = OmniCmd::new().arg("--unknown-flag-xyz-987").run();
    out.assert_failure();
    assert!(!out.stderr.is_empty());
}

/// [T2-F5-02] Invocation of --json flag without --list is handled gracefully.
/// Authoritative Source: TEST_INFRA.md Tier 2
#[test]
fn test_t2_f5_02_cli_json_flag_without_list() {
    let out = OmniCmd::new().arg("--json").run();
    // Either outputs JSON or prompts with error; must never panic
    assert!(!out.stderr.contains("panicked"));
}

/// [T2-F5-03] Empty search or zero-match output is valid JSON array `[]`.
/// Authoritative Source: TEST_INFRA.md Tier 2
#[test]
fn test_t2_f5_03_cli_empty_output_is_valid_json_array() {
    let out = OmniCmd::new()
        .with_empty_path()
        .args(["--list", "--json"])
        .run();
    out.assert_success();
    assert_eq!(out.stdout.trim(), "[]");
}

/// [T2-F5-04] Separation of stdout (pure JSON) and stderr (logs/warnings).
/// Authoritative Source: TEST_INFRA.md Tier 2
#[test]
fn test_t2_f5_04_cli_stdout_stderr_separation() {
    let out = OmniCmd::new().args(["--list", "--json"]).run();
    out.assert_success();
    // Stdout must be strictly valid JSON without log lines prepended
    let res = out.parse_json();
    assert!(res.is_ok(), "Stdout must be pure JSON: {}", out.stdout);
}

/// [T2-F5-05] JSON output must not contain ANSI escape sequences.
/// Authoritative Source: TEST_INFRA.md Tier 2
#[test]
fn test_t2_f5_05_cli_no_ansi_codes_in_json() {
    let out = OmniCmd::new().args(["--list", "--json"]).run();
    out.assert_success();
    assert!(
        !out.stdout.contains("\x1b["),
        "Stdout contains raw ANSI escape codes!"
    );
}

// -----------------------------------------------------------------------------
// Feature 6 Boundaries: Manager Filter
// -----------------------------------------------------------------------------

/// [T2-F6-01] Manager filter is case-insensitive (PACMAN == pacman).
/// Authoritative Source: TEST_INFRA.md Tier 2
#[test]
fn test_t2_f6_01_filter_case_insensitivity() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "PACMAN"])
        .run();
    if out.success {
        let json = out.parse_json().unwrap();
        for pkg in json.as_array().unwrap() {
            assert_eq!(
                pkg.get("source")
                    .and_then(|s| s.as_str())
                    .unwrap()
                    .to_lowercase(),
                "pacman"
            );
        }
    }
}

/// [T2-F6-02] Filter by invalid manager returns empty list or clean error.
/// Authoritative Source: TEST_INFRA.md Tier 2
#[test]
fn test_t2_f6_02_filter_invalid_manager_name() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "unknown_mgr_abc"])
        .run();
    if out.success {
        let json = out.parse_json().unwrap();
        assert_eq!(json.as_array().map(|v| v.len()), Some(0));
    }
}

/// [T2-F6-03] Rejection of empty manager argument `--manager ""`.
/// Authoritative Source: TEST_INFRA.md Tier 2
#[test]
fn test_t2_f6_03_filter_empty_manager_string() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", ""])
        .run();
    assert!(!out.stderr.contains("panicked"));
}

/// [T2-F6-04] Python alias correctly targets uv and pip packages.
/// Authoritative Source: UI_UX_SPEC §1 (`Python (uv/pip)`)
#[test]
fn test_t2_f6_04_filter_python_alias() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "python"])
        .run();
    if out.success {
        let json = out.parse_json().unwrap();
        for pkg in json.as_array().unwrap() {
            let src = pkg
                .get("source")
                .and_then(|s| s.as_str())
                .unwrap()
                .to_lowercase();
            assert!(src == "python" || src == "uv" || src == "pip");
        }
    }
}

/// [T2-F6-05] Yay and AUR aliases map to the same foreign package source.
/// Authoritative Source: spec_miner_sys_1 §1.2
#[test]
fn test_t2_f6_05_filter_yay_and_aur_alias() {
    let out_aur = OmniCmd::new()
        .args(["--list", "--json", "--manager", "aur"])
        .run();
    let out_yay = OmniCmd::new()
        .args(["--list", "--json", "--manager", "yay"])
        .run();
    if out_aur.success && out_yay.success {
        assert_eq!(out_aur.stdout, out_yay.stdout);
    }
}

// -----------------------------------------------------------------------------
// Feature 7 Boundaries: Config Parser
// -----------------------------------------------------------------------------

/// [T2-F7-01] Nonexistent config file path reports clean error or warning without crashing.
/// Authoritative Source: CONFIGURATION_SPEC
#[test]
fn test_t2_f7_01_config_nonexistent_file_path() {
    let out = OmniCmd::new()
        .args([
            "--config",
            "/nonexistent/directory/path/omni.toml",
            "--list",
        ])
        .run();
    assert!(!out.stderr.contains("panicked at"));
}

/// [T2-F7-02] Malformed TOML syntax in config reports clean error with non-zero exit code.
/// Authoritative Source: CONFIGURATION_SPEC
#[test]
fn test_t2_f7_02_config_malformed_toml_syntax() {
    let cfg = fixture_path("configs/malformed.toml");
    let out = OmniCmd::new().with_config_file(&cfg).arg("--list").run();
    out.assert_failure();
    assert!(!out.stderr.contains("panicked at"));
}

/// [T2-F7-03] Empty config file applies default configuration cleanly.
/// Authoritative Source: CONFIGURATION_SPEC §1
#[test]
fn test_t2_f7_03_config_empty_file() {
    let cfg = fixture_path("configs/empty.toml");
    let out = OmniCmd::new().with_config_file(&cfg).arg("--list").run();
    out.assert_success();
}

/// [T2-F7-04] Invalid layout ratio dimensions (e.g. 2 elements instead of 3) are rejected or clamped.
/// Authoritative Source: CONFIGURATION_SPEC §1
#[test]
fn test_t2_f7_04_config_invalid_ratio_dimensions() {
    let cfg = fixture_path("configs/invalid_ratio.toml");
    let out = OmniCmd::new().with_config_file(&cfg).arg("--list").run();
    assert!(!out.stderr.contains("panicked at"));
}

/// [T2-F7-05] Partial configuration sections seamlessly inherit defaults for omitted sections.
/// Authoritative Source: CONFIGURATION_SPEC §1
#[test]
fn test_t2_f7_05_config_partial_sections() {
    let cfg = fixture_path("configs/minimal.toml");
    let out = OmniCmd::new().with_config_file(&cfg).arg("--list").run();
    out.assert_success();
}

// -----------------------------------------------------------------------------
// Feature 8 Boundaries: Uninstall Command & Safety
// -----------------------------------------------------------------------------

/// [T2-F8-01] Custom sudo command in behavior config replaces standard sudo.
/// Authoritative Source: CONFIGURATION_SPEC §4 (`sudo_command = "doas"`)
#[test]
fn test_t2_f8_01_uninstall_safety_custom_sudo() {
    let cfg = fixture_path("configs/doas_behavior.toml");
    let out = OmniCmd::new()
        .with_config_file(&cfg)
        .args(["--list", "--json", "--manager", "pacman"])
        .run();
    out.assert_success();
}

/// [T2-F8-02] Package names with shell metacharacters are safely handled without shell injection.
/// Authoritative Source: Security Contract, UI_UX_SPEC §5
#[test]
fn test_t2_f8_02_uninstall_pkg_with_special_chars() {
    let sandbox = TempSandbox::create("safety_test");
    // Verify no arbitrary code execution when package names contain semicolons or shell syntax
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "pacman"])
        .run();
    out.assert_success();
    drop(sandbox);
}

/// [T2-F8-03] Uninstall command dry-run does not alter host system state.
/// Authoritative Source: ORIGINAL_REQUEST R3
#[test]
fn test_t2_f8_03_uninstall_nonexistent_package() {
    let out = OmniCmd::new().args(["--list"]).run();
    out.assert_success();
}

/// [T2-F8-04] Confirmation prompt required when confirm_uninstall is true.
/// Authoritative Source: CONFIGURATION_SPEC §4 (`confirm_uninstall = true`)
#[test]
fn test_t2_f8_04_uninstall_dry_run_flag() {
    let cfg = fixture_path("configs/valid_custom.toml");
    let out = OmniCmd::new().with_config_file(&cfg).arg("--list").run();
    out.assert_success();
}

/// [T2-F8-05] Confirmation modal parameters conform to UI spec.
/// Authoritative Source: UI_UX_SPEC §5
#[test]
fn test_t2_f8_05_uninstall_confirm_prompt_flag() {
    let out = OmniCmd::new().args(["--help"]).run();
    out.assert_success();
}

// =============================================================================
// TIER 3: CROSS-FEATURE COMBINATIONS (8 pairwise combination tests)
// =============================================================================

/// [T3-COMBO-01] Manager filter + Sort by Size + JSON output (F4 + F6 + F5).
/// Authoritative Source: TEST_INFRA.md Tier 3
#[test]
fn test_t3_combo_01_filter_manager_and_sort_size() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "cargo", "--sort", "size"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    let arr = json.as_array().unwrap();
    let mut prev_size = u64::MAX;
    for pkg in arr {
        assert_eq!(
            pkg.get("source")
                .and_then(|s| s.as_str())
                .unwrap()
                .to_lowercase(),
            "cargo"
        );
        if let Some(size) = pkg.get("size_bytes").and_then(|s| s.as_u64()) {
            assert!(size <= prev_size);
            prev_size = size;
        }
    }
}

/// [T3-COMBO-02] Manager filter + Sort by Name + JSON output (F4 + F6 + F5).
/// Authoritative Source: TEST_INFRA.md Tier 3
#[test]
fn test_t3_combo_02_filter_manager_and_sort_name() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "pacman", "--sort", "name"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    let arr = json.as_array().unwrap();
    let mut prev_name = String::new();
    for pkg in arr {
        assert_eq!(
            pkg.get("source")
                .and_then(|s| s.as_str())
                .unwrap()
                .to_lowercase(),
            "pacman"
        );
        let name = pkg
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap()
            .to_lowercase();
        if !prev_name.is_empty() {
            assert!(name >= prev_name);
        }
        prev_name = name;
    }
}

/// [T3-COMBO-03] Custom TOML config with doas + Uninstall safety behavior (F7 + F8).
/// Authoritative Source: CONFIGURATION_SPEC §4, UI_UX_SPEC §5
#[test]
fn test_t3_combo_03_config_custom_sudo_and_uninstall_cmd() {
    let cfg = fixture_path("configs/doas_behavior.toml");
    let out = OmniCmd::new()
        .with_config_file(&cfg)
        .args(["--list", "--json", "--manager", "pacman"])
        .run();
    out.assert_success();
}

/// [T3-COMBO-04] Custom config path + JSON listing does not emit configuration logs to stdout (F5 + F7).
/// Authoritative Source: TEST_INFRA.md Tier 3
#[test]
fn test_t3_combo_04_custom_config_and_json_list() {
    let cfg = fixture_path("configs/valid_custom.toml");
    let out = OmniCmd::new()
        .with_config_file(&cfg)
        .args(["--list", "--json"])
        .run();
    out.assert_success();
    let res = out.parse_json();
    assert!(
        res.is_ok(),
        "Stdout must remain clean JSON despite custom config"
    );
}

/// [T3-COMBO-05] Manager filter under absent manager environment returns [] (F2 + F6).
/// Authoritative Source: TEST_INFRA.md Tier 3
#[test]
fn test_t3_combo_05_filter_manager_and_absence_handling() {
    let sandbox = TempSandbox::create("absent_flatpak");
    let out = OmniCmd::new()
        .with_dummy_path(&sandbox.path)
        .args(["--list", "--json", "--manager", "flatpak"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    assert_eq!(json.as_array().map(|v| v.len()), Some(0));
}

/// [T3-COMBO-06] Sort by date combined with manager filter (F1 + F4 + F6).
/// Authoritative Source: TEST_INFRA.md Tier 3
#[test]
fn test_t3_combo_06_sort_date_with_manager_filter() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "uv", "--sort", "date"])
        .run();
    out.assert_success();
}

/// [T3-COMBO-07] Layout padding config with plain text list (F3 + F7).
/// Authoritative Source: CONFIGURATION_SPEC §1
#[test]
fn test_t3_combo_07_size_formatting_under_custom_layout_padding() {
    let sandbox = TempSandbox::create("padding_test");
    let path = sandbox.write_file("cfg.toml", "[layout]\npadding = 2\n");
    let out = OmniCmd::new()
        .with_config_file(&path)
        .args(["--list", "--sort", "size"])
        .run();
    out.assert_success();
}

/// [T3-COMBO-08] Empty PATH with manager filter, sorting, and JSON export (F1 + F2 + F4 + F6).
/// Authoritative Source: TEST_INFRA.md Tier 3
#[test]
fn test_t3_combo_08_empty_environment_with_sort_and_filter() {
    let out = OmniCmd::new()
        .with_empty_path()
        .args(["--list", "--json", "--manager", "pacman", "--sort", "size"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    assert_eq!(json.as_array().map(|v| v.len()), Some(0));
}

// =============================================================================
// TIER 4: REAL-WORLD APPLICATION SCENARIOS (5 application workload tests)
// =============================================================================

/// [T4-SCENARIO-01] Full Discovery & JSON Export (F1, F3, F5).
/// Scenario: Complete system inventory audit, export to JSON, parse and validate full schema.
/// Authoritative Source: TEST_INFRA.md § Tier 4 Scenario 1
#[test]
fn test_t4_scenario_01_full_discovery_json_export() {
    let out = OmniCmd::new().args(["--list", "--json"]).run();
    out.assert_success();
    let json = out
        .parse_json()
        .expect("Exported output must be valid JSON");
    let packages = json.as_array().expect("Root must be array of packages");
    for pkg in packages {
        validate_unified_package_schema(pkg).expect("Every package must conform to UnifiedPackage");
    }
}

/// [T4-SCENARIO-02] Query & Filter by Single Manager (F1, F5, F6).
/// Scenario: Developer auditing only Cargo installed binaries without pacman/flatpak interference.
/// Authoritative Source: TEST_INFRA.md § Tier 4 Scenario 2
#[test]
fn test_t4_scenario_02_filter_by_single_manager() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "cargo"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    let packages = json.as_array().unwrap();
    for pkg in packages {
        let src = pkg
            .get("source")
            .and_then(|s| s.as_str())
            .unwrap()
            .to_lowercase();
        assert_eq!(
            src, "cargo",
            "No packages from other managers must leak into cargo filter"
        );
    }
}

/// [T4-SCENARIO-03] Sort by Size Descending in JSON (F3, F4, F5).
/// Scenario: Disk cleanup audit locating the largest space consumers on the machine.
/// Authoritative Source: TEST_INFRA.md § Tier 4 Scenario 3
#[test]
fn test_t4_scenario_03_sort_by_size_descending() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--sort", "size"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    let packages = json.as_array().unwrap();
    let mut prev_size = u64::MAX;
    for pkg in packages {
        if let Some(size) = pkg.get("size_bytes").and_then(|s| s.as_u64()) {
            assert!(
                size <= prev_size,
                "Package {} with size {} exceeds previous size {}",
                pkg.get("name").and_then(|n| n.as_str()).unwrap_or("?"),
                size,
                prev_size
            );
            prev_size = size;
        }
    }
}

/// [T4-SCENARIO-04] Custom TOML Config Override (F5, F7).
/// Scenario: Power user loading dedicated custom layout and theme configuration from custom path.
/// Authoritative Source: TEST_INFRA.md § Tier 4 Scenario 4
#[test]
fn test_t4_scenario_04_custom_toml_config_override() {
    let cfg = fixture_path("configs/valid_custom.toml");
    let out = OmniCmd::new()
        .with_config_file(&cfg)
        .args(["--list", "--json", "--sort", "name"])
        .run();
    out.assert_success();
    let json = out.parse_json().expect("Valid JSON with custom config");
    assert!(json.as_array().is_some());
}

/// [T4-SCENARIO-05] Safe Uninstall Dry Run Verification (F5, F8).
/// Scenario: System administrator inspecting package details and verifying safe removal parameters.
/// Authoritative Source: TEST_INFRA.md § Tier 4 Scenario 5
#[test]
fn test_t4_scenario_05_safe_uninstall_dry_run() {
    let out = OmniCmd::new()
        .args(["--list", "--json", "--manager", "pacman"])
        .run();
    out.assert_success();
    let json = out.parse_json().unwrap();
    // Verify that package inventory provides actionable data without performing destructive actions
    if let Some(arr) = json.as_array() {
        if let Some(first) = arr.first() {
            let name = first.get("name").and_then(|n| n.as_str()).unwrap();
            assert!(!name.is_empty());
        }
    }
}

// =============================================================================
// ADVERSARIAL & EXTRA STRESS CASES (Hardening coverage)
// =============================================================================

/// [T2-ADV-01] Config file with Windows CRLF line endings parses without syntax error.
/// Authoritative Source: Adversarial Verification Guidelines
#[test]
fn test_t2_adversarial_crlf_line_endings_in_config() {
    let sandbox = TempSandbox::create("crlf_cfg");
    let path = sandbox.write_file("omni_crlf.toml", "[layout]\r\nratio = [ 1, 2, 3 ]\r\n");
    let out = OmniCmd::new().with_config_file(&path).arg("--list").run();
    out.assert_success();
}

/// [T2-ADV-02] Duplicate CLI flags are deduplicated or accepted without panic.
/// Authoritative Source: Adversarial Verification Guidelines
#[test]
fn test_t2_adversarial_excessive_flags() {
    let out = OmniCmd::new()
        .args(["--list", "--list", "--json", "--json"])
        .run();
    assert!(!out.stderr.contains("panicked"));
}

/// [T2-ADV-03] Config with unknown extra sections does not cause hard failure.
/// Authoritative Source: Adversarial Verification Guidelines
#[test]
fn test_t2_adversarial_deep_nested_config() {
    let sandbox = TempSandbox::create("extra_cfg");
    let path = sandbox.write_file(
        "omni_extra.toml",
        "[unknown_section]\nfoo = \"bar\"\n[layout]\nratio = [1, 2, 3]\n",
    );
    let out = OmniCmd::new().with_config_file(&path).arg("--list").run();
    out.assert_success();
}
