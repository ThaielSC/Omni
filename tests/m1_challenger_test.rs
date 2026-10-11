//! Milestone 1 Iteration 2 Empirical Challenger Test Suite
//! Adversarial verification of `parse_hex_color` and `format_bytes_compact`.

use omni::config::parse_hex_color;
use omni::utils::format::format_bytes_compact;
use ratatui::style::Color;

// =============================================================================
// CHALLENGE 1: parse_hex_color Multi-byte UTF-8 and Adversarial Input Stress
// =============================================================================

#[test]
fn test_adversarial_parse_hex_color_multibyte_utf8() {
    let multibyte_cases = [
        // 4-byte UTF-8 emoji inputs
        "#🔥12",        // 4 bytes + 2 ASCII = 6 bytes
        "#12🔥",        // 2 ASCII + 4 bytes = 6 bytes
        "#🔥",          // 4 bytes
        "#🚀🚀",        // 8 bytes
        "#\u{1F600}12", // 4 bytes + 2 ASCII = 6 bytes
        "#12\u{1F600}", // 2 ASCII + 4 bytes = 6 bytes
        "#🦀",          // 4 bytes
        "#❤️12",        // 3-byte heart + 3-byte variation selector
        // 2-byte UTF-8 accented inputs
        "#a\u{00e9}bcd", // 1 + 2 + 3 = 6 bytes
        "#ééé",          // 2 + 2 + 2 = 6 bytes
        "#ñ1",           // 2 + 1 = 3 bytes
        "#1ñ",           // 1 + 2 = 3 bytes
        "#ü1234",        // 2 + 4 = 6 bytes
        "#1234ü",        // 4 + 2 = 6 bytes
        "#ßßß",          // 2 + 2 + 2 = 6 bytes
        "#αβγ",          // Greek: 2 + 2 + 2 = 6 bytes
        "#абв",          // Cyrillic: 2 + 2 + 2 = 6 bytes
        // 3-byte UTF-8 inputs
        "#\u{20ac}", // Euro sign: 3 bytes
        "#€12",      // 3 + 2 = 5 bytes
        "#€€",       // 3 + 3 = 6 bytes
        "#中1",      // CJK 3 bytes + 1 = 4 bytes
        "#中文",     // CJK 3 + 3 = 6 bytes
        "#日本語",   // CJK 3 + 3 + 3 = 9 bytes
        // Non-prefixed multibyte inputs
        "🔥12",
        "12🔥",
        "a\u{00e9}bcd",
        "ééé",
        "ñ1",
        "1ñ",
        "€€",
        "中文",
        // Control & Special characters
        "#\012345", // Null byte in 6-byte string
        "#\t12345", // Tab in 6-byte string
        "#\n12345", // Newline
        "# 12345",  // Leading space in raw
        "#12345 ",  // Trailing space in raw
        "   #🔥12   ",
        "   🔥12   ",
    ];

    for case in multibyte_cases {
        let res = parse_hex_color(case);
        assert!(
            res.is_err(),
            "Expected parse_hex_color to reject multibyte UTF-8 input '{}' with Err, got {:?}",
            case,
            res
        );
    }
}

#[test]
fn test_adversarial_parse_hex_color_invalid_ascii() {
    let invalid_ascii_cases = [
        "",
        "   ",
        "\t\n\r",
        "#",
        "##",
        "###",
        "####",
        "#####",
        "######",
        "#######",
        "#1",
        "#12",
        "#1234",
        "#12345",
        "#1234567",
        "#12345678",
        "1",
        "12",
        "1234",
        "12345",
        "1234567",
        "12345678",
        "#gggggg",
        "#GGGGGG",
        "#12345g",
        "#g12345",
        "#zzz",
        "#12z",
        "#z12",
        "gggggg",
        "zzz",
        "0x123456",
        "#0x123456",
        "-#123456",
        "+#123456",
        "rgb(255, 255, 255)",
        "hsl(100, 50%, 50%)",
        "unknown_color_name",
        "not-a-color",
    ];

    for case in invalid_ascii_cases {
        let res = parse_hex_color(case);
        assert!(
            res.is_err(),
            "Expected parse_hex_color to reject invalid ASCII input '{}' with Err, got {:?}",
            case,
            res
        );
    }
}

#[test]
fn test_adversarial_parse_hex_color_valid_variants() {
    // Standard 6-digit hex
    assert_eq!(parse_hex_color("#000000").unwrap(), Color::Rgb(0, 0, 0));
    assert_eq!(
        parse_hex_color("#ffffff").unwrap(),
        Color::Rgb(255, 255, 255)
    );
    assert_eq!(
        parse_hex_color("#FFFFFF").unwrap(),
        Color::Rgb(255, 255, 255)
    );
    assert_eq!(
        parse_hex_color("#7daea3").unwrap(),
        Color::Rgb(0x7d, 0xae, 0xa3)
    );
    assert_eq!(
        parse_hex_color("#ea6962").unwrap(),
        Color::Rgb(0xea, 0x69, 0x62)
    );
    assert_eq!(parse_hex_color("000000").unwrap(), Color::Rgb(0, 0, 0));
    assert_eq!(
        parse_hex_color("ffffff").unwrap(),
        Color::Rgb(255, 255, 255)
    );
    assert_eq!(
        parse_hex_color("7daea3").unwrap(),
        Color::Rgb(0x7d, 0xae, 0xa3)
    );

    // 3-digit shorthand
    assert_eq!(parse_hex_color("#000").unwrap(), Color::Rgb(0, 0, 0));
    assert_eq!(parse_hex_color("#fff").unwrap(), Color::Rgb(255, 255, 255));
    assert_eq!(parse_hex_color("#FFF").unwrap(), Color::Rgb(255, 255, 255));
    assert_eq!(parse_hex_color("#f0a").unwrap(), Color::Rgb(255, 0, 170));
    assert_eq!(parse_hex_color("000").unwrap(), Color::Rgb(0, 0, 0));
    assert_eq!(parse_hex_color("fff").unwrap(), Color::Rgb(255, 255, 255));
    assert_eq!(parse_hex_color("f0a").unwrap(), Color::Rgb(255, 0, 170));

    // Named ANSI colors (case-insensitive)
    assert_eq!(parse_hex_color("red").unwrap(), Color::Red);
    assert_eq!(parse_hex_color("RED").unwrap(), Color::Red);
    assert_eq!(parse_hex_color("Green").unwrap(), Color::Green);
    assert_eq!(parse_hex_color("BLUE").unwrap(), Color::Blue);
    assert_eq!(parse_hex_color("reset").unwrap(), Color::Reset);
    assert_eq!(parse_hex_color("black").unwrap(), Color::Black);
    assert_eq!(parse_hex_color("yellow").unwrap(), Color::Yellow);
    assert_eq!(parse_hex_color("magenta").unwrap(), Color::Magenta);
    assert_eq!(parse_hex_color("cyan").unwrap(), Color::Cyan);
    assert_eq!(parse_hex_color("gray").unwrap(), Color::Gray);
    assert_eq!(parse_hex_color("grey").unwrap(), Color::Gray);
    assert_eq!(parse_hex_color("darkgray").unwrap(), Color::DarkGray);
    assert_eq!(parse_hex_color("darkgrey").unwrap(), Color::DarkGray);
    assert_eq!(parse_hex_color("lightred").unwrap(), Color::LightRed);
    assert_eq!(parse_hex_color("lightgreen").unwrap(), Color::LightGreen);
    assert_eq!(parse_hex_color("lightyellow").unwrap(), Color::LightYellow);
    assert_eq!(parse_hex_color("lightblue").unwrap(), Color::LightBlue);
    assert_eq!(
        parse_hex_color("lightmagenta").unwrap(),
        Color::LightMagenta
    );
    assert_eq!(parse_hex_color("lightcyan").unwrap(), Color::LightCyan);
    assert_eq!(parse_hex_color("white").unwrap(), Color::White);
}

// =============================================================================
// CHALLENGE 2: format_bytes_compact Width Invariant (len <= 5) Across 0B to 100TiB
// =============================================================================

#[test]
fn test_compact_format_none_and_zero() {
    assert_eq!(format_bytes_compact(None), "-");
    assert_eq!(format_bytes_compact(Some(0)), "0B");
}

#[test]
fn test_compact_format_sweep_0_to_2048() {
    for b in 0..=2048 {
        let s = format_bytes_compact(Some(b));
        assert!(
            s.len() <= 5,
            "format_bytes_compact(Some({})) produced '{}' (len {}) > 5",
            b,
            s,
            s.len()
        );
    }
}

#[test]
fn test_compact_format_boundary_windows() {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;
    const TIB: u64 = 1024 * GIB;

    // Check ±2048 window around every transition and power boundary
    let centers = [
        KIB,
        10 * KIB,
        99 * KIB,
        (99.95 * 1024.0) as u64,
        100 * KIB,
        500 * KIB,
        1000 * KIB,
        MIB,
        10 * MIB,
        99 * MIB,
        (99.95 * 1024.0 * 1024.0) as u64,
        100 * MIB,
        500 * MIB,
        1000 * MIB,
        GIB,
        10 * GIB,
        99 * GIB,
        (99.95 * 1024.0 * 1024.0 * 1024.0) as u64,
        100 * GIB,
        500 * GIB,
        1000 * GIB,
        TIB,
        10 * TIB,
        50 * TIB,
        99 * TIB,
        (99.95 * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64,
        100 * TIB,
    ];

    for center in centers {
        let start = center.saturating_sub(2048);
        let end = center.saturating_add(2048);
        for b in start..=end {
            let s = format_bytes_compact(Some(b));
            assert!(
                s.len() <= 5,
                "format_bytes_compact(Some({})) produced '{}' (len {}) > 5",
                b,
                s,
                s.len()
            );
        }
    }
}

#[test]
fn test_compact_format_dense_geometric_sweep_to_100_tib() {
    const TIB: f64 = 1024.0 * 1024.0 * 1024.0 * 1024.0;
    let max_bytes = 100.0 * TIB;

    // Step geometrically: curr *= 1.0002 (~160,000 steps covering 1 to 100 TiB)
    let mut curr = 1.0f64;
    let factor = 1.0002f64;

    while curr <= max_bytes {
        let b = curr as u64;
        let s = format_bytes_compact(Some(b));
        assert!(
            s.len() <= 5,
            "format_bytes_compact(Some({})) produced '{}' (len {}) > 5",
            b,
            s,
            s.len()
        );
        curr *= factor;
    }

    // Explicit test at exactly 100 TiB
    let exactly_100_tib = (100.0 * TIB) as u64;
    let s = format_bytes_compact(Some(exactly_100_tib));
    assert!(
        s.len() <= 5,
        "format_bytes_compact(100 TiB) produced '{}' (len {}) > 5",
        s,
        s.len()
    );
    assert_eq!(s, "100T");
}

#[test]
fn test_compact_format_pseudo_random_fuzzing() {
    const TIB: u64 = 1024 * 1024 * 1024 * 1024;
    let max_bytes = 100 * TIB;

    // Simple LCG PRNG for reproducible adversarial fuzzing
    let mut state: u64 = 0xdeadbeefcafebabe;
    let mut lcg = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        state
    };

    // 200,000 randomized probes across the entire range
    for _ in 0..200_000 {
        let raw = lcg();
        let b = raw % (max_bytes + 1);
        let s = format_bytes_compact(Some(b));
        assert!(
            s.len() <= 5,
            "format_bytes_compact(Some({})) produced '{}' (len {}) > 5",
            b,
            s,
            s.len()
        );
    }
}
