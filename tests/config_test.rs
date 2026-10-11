//! Empirical Challenge and Stress Test Suite for Milestone 1 Configuration and Keymaps
//! Authoritative references:
//! - docs/CONFIGURATION_SPEC.md
//! - docs/MANAGERS_SPEC.md
//! - .agents/teamwork/PROJECT.md

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use omni::config::{
    Action, KeyBindings, KeymapConfig, LayoutConfig, OmniConfig, OutputFormat, ThemeColors,
    parse_hex_color, parse_key_event, parse_keymap_toml, parse_managers_toml, parse_theme_toml,
};
use ratatui::style::Color;
use ratatui::widgets::BorderType;
use std::panic;

// =============================================================================
// AREA 1: OmniConfig Deserialization with Malformed / Incomplete TOML
// =============================================================================

#[test]
fn test_stress_empty_and_whitespace_toml() {
    // Completely empty string should produce default OmniConfig
    let cfg_empty = OmniConfig::from_toml_str("").expect("Empty TOML must deserialize cleanly");
    assert_eq!(cfg_empty, OmniConfig::default());

    // Whitespace only
    let cfg_ws = OmniConfig::from_toml_str("   \n\t  \r\n   ")
        .expect("Whitespace-only TOML must deserialize cleanly");
    assert_eq!(cfg_ws, OmniConfig::default());
}

#[test]
fn test_stress_incomplete_toml_subsections() {
    // Only layout section
    let toml_layout_only = r#"
        [layout]
        border_style = "double"
    "#;
    let cfg = OmniConfig::from_toml_str(toml_layout_only).expect("Partial layout must succeed");
    assert_eq!(cfg.layout.border_style, "double");
    assert_eq!(cfg.layout.ratio, [1, 2, 3]); // default retained
    assert!(cfg.layout.show_managers); // default retained
    assert_eq!(cfg.behavior.sudo_command, "sudo"); // default retained
    assert_eq!(cfg.theme.name, "gruvbox-material"); // default retained

    // Only behavior section
    let toml_behavior_only = r#"
        [behavior]
        sudo_command = "pkexec"
        confirm_uninstall = false
    "#;
    let cfg2 =
        OmniConfig::from_toml_str(toml_behavior_only).expect("Partial behavior must succeed");
    assert_eq!(cfg2.behavior.sudo_command, "pkexec");
    assert!(!cfg2.behavior.confirm_uninstall);
    assert!(cfg2.behavior.preload_disk_sizes); // default retained
    assert_eq!(cfg2.layout.ratio, [1, 2, 3]); // default retained

    // Only keymap section with a single binding
    let toml_keymap_only = r#"
        [keymap]
        quit = "X"
    "#;
    let cfg3 = OmniConfig::from_toml_str(toml_keymap_only).expect("Partial keymap must succeed");
    assert_eq!(cfg3.keymap.quit.as_slice(), &["X"]);
    assert_eq!(cfg3.keymap.help.as_slice(), &["?"]); // default retained
    assert_eq!(cfg3.keymap.down.as_slice(), &["j", "<Down>"]); // default retained

    // Only theme colors section
    let toml_theme_only = r##"
        [theme]
        name = "nord"
        [theme.colors]
        bg = "#2e3440"
    "##;
    let cfg4 = OmniConfig::from_toml_str(toml_theme_only).expect("Partial theme must succeed");
    assert_eq!(cfg4.theme.name, "nord");
    assert_eq!(cfg4.theme.colors.bg, "#2e3440");
    assert_eq!(cfg4.theme.colors.fg, "#d4be98"); // default retained
}

#[test]
fn test_stress_malformed_toml_syntax_reports_clean_error() {
    let malformed_cases = [
        "[layout\nratio = [1, 2, 3]",              // Unclosed table header
        "[layout]\nratio = [1, 2,",                // Unclosed array
        "[layout]\nborder_style = \"rounded",      // Unclosed string literal
        "???? not valid toml ????",                // Syntax gibberish
        "[layout]\nratio = [1, 2, 3]\n[layout]\n", // Duplicate table definition
        "[layout]\nratio = \"invalid_type\"",      // Type mismatch (string instead of array)
        "[layout]\nshow_managers = 12345",         // Type mismatch (number instead of bool)
        "[layout]\npadding = -10",                 // Type mismatch (negative u16)
        "[behavior]\nconfirm_uninstall = \"yes\"", // Type mismatch (string instead of bool)
    ];

    for case in malformed_cases {
        let res = OmniConfig::from_toml_str(case);
        assert!(
            res.is_err(),
            "Expected malformed TOML to return Err, but got Ok for input: {}",
            case
        );
    }
}

#[test]
fn test_stress_layout_ratio_edge_cases() {
    // 1. Dynamic 0 elements -> defaults to [1, 2, 3]
    let cfg: OmniConfig = OmniConfig::from_toml_str("[layout]\nratio = []").unwrap();
    assert_eq!(cfg.layout.ratio, [1, 2, 3]);

    // 2. Dynamic 1 element -> [5, 2, 3]
    let cfg: OmniConfig = OmniConfig::from_toml_str("[layout]\nratio = [5]").unwrap();
    assert_eq!(cfg.layout.ratio, [5, 2, 3]);

    // 3. Dynamic 2 elements -> [4, 6, 3]
    let cfg: OmniConfig = OmniConfig::from_toml_str("[layout]\nratio = [4, 6]").unwrap();
    assert_eq!(cfg.layout.ratio, [4, 6, 3]);

    // 4. Dynamic > 3 elements -> takes first 3
    let cfg: OmniConfig = OmniConfig::from_toml_str("[layout]\nratio = [2, 3, 4, 5, 6]").unwrap();
    assert_eq!(cfg.layout.ratio, [2, 3, 4]);

    // 5. Zero values in dynamic array clamped to 1
    let cfg: OmniConfig = OmniConfig::from_toml_str("[layout]\nratio = [0, 5]").unwrap();
    assert_eq!(cfg.layout.ratio, [1, 5, 3]);

    // 6. Degenerate ratio [0, 0, 0] must not panic in column_constraints
    let degenerate = LayoutConfig {
        ratio: [0, 0, 0],
        show_managers: false,
        show_details: false,
        ..Default::default()
    };
    let constraints = degenerate.column_constraints();
    assert_eq!(constraints.len(), 1);

    // 7. Large ratios
    let large = LayoutConfig {
        ratio: [1000, 2000, 3000],
        ..Default::default()
    };
    let constraints_large = large.column_constraints();
    assert_eq!(constraints_large.len(), 3);
}

#[test]
fn test_stress_border_styles_variants() {
    let variants = [
        ("rounded", BorderType::Rounded),
        ("ROUNDED", BorderType::Rounded),
        ("plain", BorderType::Plain),
        ("PLAIN", BorderType::Plain),
        ("thick", BorderType::Thick),
        ("THICK", BorderType::Thick),
        ("double", BorderType::Double),
        ("DOUBLE", BorderType::Double),
        // Fallback variants
        ("none", BorderType::Plain),
        ("unknown", BorderType::Plain),
        ("", BorderType::Plain),
        ("fancy_border", BorderType::Plain),
    ];

    for (style_str, expected) in variants {
        let layout = LayoutConfig {
            border_style: style_str.to_string(),
            ..Default::default()
        };
        assert_eq!(
            layout.border_type(),
            expected,
            "Failed for border_style '{}'",
            style_str
        );
    }
}

#[test]
fn test_stress_theme_and_keymap_standalone_loaders() {
    // 1. Wrapped theme TOML
    let wrapped_theme = r##"
        [theme]
        name = "catppuccin"
        [theme.colors]
        bg = "#1e1e2e"
    "##;
    let t1 = parse_theme_toml(wrapped_theme).expect("Wrapped theme TOML should parse");
    assert_eq!(t1.name, "catppuccin");
    assert_eq!(t1.colors.bg, "#1e1e2e");

    // 2. Unwrapped (root-level) theme TOML
    let bare_theme = r##"
        name = "tokyonight"
        [colors]
        bg = "#1a1b26"
    "##;
    let t2 = parse_theme_toml(bare_theme).expect("Bare theme TOML should parse");
    assert_eq!(t2.name, "tokyonight");
    assert_eq!(t2.colors.bg, "#1a1b26");

    // 3. Wrapped keymap TOML
    let wrapped_keymap = r#"
        [keymap]
        quit = "<C-q>"
    "#;
    let k1 = parse_keymap_toml(wrapped_keymap).expect("Wrapped keymap TOML should parse");
    assert_eq!(k1.quit.as_slice(), &["<C-q>"]);

    // 4. Unwrapped (root-level) keymap TOML
    let bare_keymap = r#"
        quit = ["q", "<C-c>"]
        help = "F1"
    "#;
    let k2 = parse_keymap_toml(bare_keymap).expect("Bare keymap TOML should parse");
    assert_eq!(k2.quit.as_slice(), &["q", "<C-c>"]);
    assert_eq!(k2.help.as_slice(), &["F1"]);
}

// =============================================================================
// AREA 2: Keymap Notation Parser Stress Testing
// =============================================================================

#[test]
fn test_stress_keymap_notation_single_chars() {
    // Alphabetic lowercase
    for ch in 'a'..='z' {
        let s = ch.to_string();
        let ev = parse_key_event(&s).unwrap_or_else(|_| panic!("Failed parsing '{}'", s));
        assert_eq!(ev.code, KeyCode::Char(ch));
        assert_eq!(ev.modifiers, KeyModifiers::NONE);
    }

    // Alphabetic uppercase (must attach SHIFT)
    for ch in 'A'..='Z' {
        let s = ch.to_string();
        let ev = parse_key_event(&s).unwrap_or_else(|_| panic!("Failed parsing '{}'", s));
        assert_eq!(ev.code, KeyCode::Char(ch));
        assert_eq!(ev.modifiers, KeyModifiers::SHIFT);
    }

    // Digits
    for ch in '0'..='9' {
        let s = ch.to_string();
        let ev = parse_key_event(&s).unwrap_or_else(|_| panic!("Failed parsing '{}'", s));
        assert_eq!(ev.code, KeyCode::Char(ch));
        assert_eq!(ev.modifiers, KeyModifiers::NONE);
    }

    // Punctuation and symbols
    let symbols = [
        "/", "?", "-", "+", ".", ",", ":", ";", "=", "_", "!", "@", "#", "$", "%", "^", "&", "*",
    ];
    for sym in symbols {
        let ev = parse_key_event(sym).unwrap_or_else(|_| panic!("Failed parsing symbol '{}'", sym));
        assert_eq!(ev.code, KeyCode::Char(sym.chars().next().unwrap()));
    }

    // Raw space character
    let ev_space = parse_key_event(" ").expect("Single space must parse");
    assert_eq!(ev_space.code, KeyCode::Char(' '));
    assert_eq!(ev_space.modifiers, KeyModifiers::NONE);
}

#[test]
fn test_stress_keymap_notation_bracketed_special_keys() {
    let test_vectors = [
        ("<Left>", KeyCode::Left, KeyModifiers::NONE),
        ("<Right>", KeyCode::Right, KeyModifiers::NONE),
        ("<Up>", KeyCode::Up, KeyModifiers::NONE),
        ("<Down>", KeyCode::Down, KeyModifiers::NONE),
        ("<Home>", KeyCode::Home, KeyModifiers::NONE),
        ("<End>", KeyCode::End, KeyModifiers::NONE),
        ("<PageUp>", KeyCode::PageUp, KeyModifiers::NONE),
        ("<pageup>", KeyCode::PageUp, KeyModifiers::NONE),
        ("<pgup>", KeyCode::PageUp, KeyModifiers::NONE),
        ("<PageDown>", KeyCode::PageDown, KeyModifiers::NONE),
        ("<pagedown>", KeyCode::PageDown, KeyModifiers::NONE),
        ("<pgdn>", KeyCode::PageDown, KeyModifiers::NONE),
        ("<Tab>", KeyCode::Tab, KeyModifiers::NONE),
        ("<tab>", KeyCode::Tab, KeyModifiers::NONE),
        ("<BackTab>", KeyCode::BackTab, KeyModifiers::NONE),
        ("<backtab>", KeyCode::BackTab, KeyModifiers::NONE),
        ("<Esc>", KeyCode::Esc, KeyModifiers::NONE),
        ("<esc>", KeyCode::Esc, KeyModifiers::NONE),
        ("<escape>", KeyCode::Esc, KeyModifiers::NONE),
        ("<Enter>", KeyCode::Enter, KeyModifiers::NONE),
        ("<return>", KeyCode::Enter, KeyModifiers::NONE),
        ("<ret>", KeyCode::Enter, KeyModifiers::NONE),
        ("<Backspace>", KeyCode::Backspace, KeyModifiers::NONE),
        ("<bs>", KeyCode::Backspace, KeyModifiers::NONE),
        ("<Delete>", KeyCode::Delete, KeyModifiers::NONE),
        ("<del>", KeyCode::Delete, KeyModifiers::NONE),
        ("<Insert>", KeyCode::Insert, KeyModifiers::NONE),
        ("<ins>", KeyCode::Insert, KeyModifiers::NONE),
        ("<Space>", KeyCode::Char(' '), KeyModifiers::NONE),
        ("<space>", KeyCode::Char(' '), KeyModifiers::NONE),
        ("<spc>", KeyCode::Char(' '), KeyModifiers::NONE),
        ("<spacebar>", KeyCode::Char(' '), KeyModifiers::NONE),
        ("<null>", KeyCode::Null, KeyModifiers::NONE),
    ];

    for (notation, expected_code, expected_mods) in test_vectors {
        let ev = parse_key_event(notation)
            .unwrap_or_else(|_| panic!("Failed parsing notation '{}'", notation));
        assert_eq!(ev.code, expected_code, "Mismatch code for '{}'", notation);
        assert_eq!(
            ev.modifiers, expected_mods,
            "Mismatch mods for '{}'",
            notation
        );
    }
}

#[test]
fn test_stress_keymap_notation_function_keys() {
    for n in 1..=12 {
        let notation = format!("<F{}>", n);
        let ev = parse_key_event(&notation)
            .unwrap_or_else(|_| panic!("Failed parsing function key '{}'", notation));
        assert_eq!(ev.code, KeyCode::F(n));
        assert_eq!(ev.modifiers, KeyModifiers::NONE);

        let lower_notation = format!("<f{}>", n);
        let ev_lower = parse_key_event(&lower_notation)
            .unwrap_or_else(|_| panic!("Failed parsing lower function key '{}'", lower_notation));
        assert_eq!(ev_lower.code, KeyCode::F(n));
    }

    // Function keys with modifiers
    let ev = parse_key_event("<C-F5>").expect("Ctrl-F5 should parse");
    assert_eq!(ev.code, KeyCode::F(5));
    assert_eq!(ev.modifiers, KeyModifiers::CONTROL);
}

#[test]
fn test_stress_keymap_notation_modifier_chords() {
    let chord_vectors = [
        ("<C-c>", KeyCode::Char('c'), KeyModifiers::CONTROL),
        ("<c-c>", KeyCode::Char('c'), KeyModifiers::CONTROL),
        ("<Ctrl-c>", KeyCode::Char('c'), KeyModifiers::CONTROL),
        ("<control-c>", KeyCode::Char('c'), KeyModifiers::CONTROL),
        ("<A-x>", KeyCode::Char('x'), KeyModifiers::ALT),
        ("<Alt-x>", KeyCode::Char('x'), KeyModifiers::ALT),
        ("<M-x>", KeyCode::Char('x'), KeyModifiers::ALT),
        ("<Meta-x>", KeyCode::Char('x'), KeyModifiers::ALT),
        ("<S-Tab>", KeyCode::BackTab, KeyModifiers::SHIFT),
        ("<Shift-Tab>", KeyCode::BackTab, KeyModifiers::SHIFT),
        ("<Super-q>", KeyCode::Char('q'), KeyModifiers::SUPER),
        ("<Cmd-q>", KeyCode::Char('q'), KeyModifiers::SUPER),
        ("<Win-q>", KeyCode::Char('q'), KeyModifiers::SUPER),
        // Chords with minus / plus symbols
        ("<C-->", KeyCode::Char('-'), KeyModifiers::CONTROL),
        ("<C-+>", KeyCode::Char('+'), KeyModifiers::CONTROL),
        // Space chords
        ("<C-Space>", KeyCode::Char(' '), KeyModifiers::CONTROL),
        ("<A-Space>", KeyCode::Char(' '), KeyModifiers::ALT),
        // Multi-modifier chords
        (
            "<C-A-x>",
            KeyCode::Char('x'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        ),
        (
            "<Ctrl-Alt-Delete>",
            KeyCode::Delete,
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        ),
        (
            "<C-S-c>",
            KeyCode::Char('C'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ),
    ];

    for (notation, expected_code, expected_mods) in chord_vectors {
        let ev = parse_key_event(notation)
            .unwrap_or_else(|_| panic!("Failed parsing chord '{}'", notation));
        assert_eq!(
            ev.code, expected_code,
            "Mismatch code for chord '{}'",
            notation
        );
        assert_eq!(
            ev.modifiers, expected_mods,
            "Mismatch mods for chord '{}'",
            notation
        );
    }
}

#[test]
fn test_stress_key_matching_semantics() {
    let keymap = KeymapConfig::default();

    // 1. Release events must NEVER match
    let release_q = KeyEvent::new_with_kind(
        KeyCode::Char('q'),
        KeyModifiers::NONE,
        KeyEventKind::Release,
    );
    assert!(!keymap.matches(Action::Quit, &release_q));

    // 2. Control chord case and shift semantics:
    // <C-c> (default) matches lowercase 'c' with CONTROL.
    let ctrl_c_lower = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert!(keymap.matches(Action::Quit, &ctrl_c_lower));

    // Notice: crossterm event with uppercase 'C' is normalized with SHIFT modifier.
    // Therefore, <C-c> does NOT match Ctrl+Shift+c (or Char('C') with CONTROL),
    // while an explicit <C-C> binding DOES match it.
    let custom_keymap = KeymapConfig {
        quit: KeyBindings::new(["<C-C>"]),
        ..Default::default()
    };
    let ctrl_c_with_shift = KeyEvent::new(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    let ctrl_c_upper = KeyEvent::new(KeyCode::Char('C'), KeyModifiers::CONTROL);
    assert!(custom_keymap.matches(Action::Quit, &ctrl_c_with_shift));
    assert!(custom_keymap.matches(Action::Quit, &ctrl_c_upper));

    // 3. Navigation case sensitivity ('j' != 'J')
    let j_lower = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    let j_upper = KeyEvent::new(KeyCode::Char('J'), KeyModifiers::SHIFT);
    assert!(keymap.matches(Action::Down, &j_lower));
    assert!(!keymap.matches(Action::Down, &j_upper));

    // 4. Tab and BackTab interoperability
    let plain_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    let shift_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT);
    let backtab = KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE);

    // FocusNext is <Tab>
    assert!(keymap.matches(Action::FocusNext, &plain_tab));
    assert!(!keymap.matches(Action::FocusNext, &shift_tab));
    assert!(!keymap.matches(Action::FocusNext, &backtab));

    // FocusPrev is <BackTab>
    assert!(keymap.matches(Action::FocusPrev, &backtab));
    assert!(keymap.matches(Action::FocusPrev, &shift_tab));
    assert!(!keymap.matches(Action::FocusPrev, &plain_tab));
}

#[test]
fn test_stress_keymap_notation_malformed_inputs() {
    let malformed_inputs = [
        "",                    // Empty string
        "   ",                 // Whitespace only
        "<>",                  // Empty angle brackets
        "<UnknownModifier-c>", // Invalid modifier
        "<C-UnknownKeyName>",  // Invalid key name
        "<F0>",                // Out of range F-key
        "<F13>",               // Out of range F-key
        "<F99>",               // Out of range F-key
        "<--",                 // Broken chord syntax
    ];

    for input in malformed_inputs {
        let res = parse_key_event(input);
        assert!(
            res.is_err(),
            "Expected parse_key_event to return Err for malformed input '{}', got {:?}",
            input,
            res
        );
    }
}

// =============================================================================
// AREA 3: Theme Color Parser with Valid, Named, and Invalid Hex Strings
// =============================================================================

#[test]
fn test_stress_theme_colors_valid_hex_and_named() {
    // 6-digit hex with hash
    assert_eq!(
        parse_hex_color("#282828").unwrap(),
        Color::Rgb(0x28, 0x28, 0x28)
    );
    assert_eq!(
        parse_hex_color("#d4be98").unwrap(),
        Color::Rgb(0xd4, 0xbe, 0x98)
    );
    assert_eq!(
        parse_hex_color("#ffffff").unwrap(),
        Color::Rgb(255, 255, 255)
    );
    assert_eq!(parse_hex_color("#000000").unwrap(), Color::Rgb(0, 0, 0));

    // 6-digit hex without hash
    assert_eq!(
        parse_hex_color("282828").unwrap(),
        Color::Rgb(0x28, 0x28, 0x28)
    );
    assert_eq!(
        parse_hex_color("ffffff").unwrap(),
        Color::Rgb(255, 255, 255)
    );

    // 3-digit shorthand with hash (#rgb -> r*17, g*17, b*17)
    assert_eq!(parse_hex_color("#fff").unwrap(), Color::Rgb(255, 255, 255));
    assert_eq!(parse_hex_color("#000").unwrap(), Color::Rgb(0, 0, 0));
    assert_eq!(parse_hex_color("#f0a").unwrap(), Color::Rgb(255, 0, 170));

    // 3-digit shorthand without hash
    assert_eq!(parse_hex_color("fff").unwrap(), Color::Rgb(255, 255, 255));

    // Case insensitivity
    assert_eq!(
        parse_hex_color("#D4BE98").unwrap(),
        Color::Rgb(0xd4, 0xbe, 0x98)
    );
    assert_eq!(
        parse_hex_color("#AaBbCc").unwrap(),
        Color::Rgb(0xaa, 0xbb, 0xcc)
    );

    // Named ANSI colors
    let named = [
        ("black", Color::Black),
        ("red", Color::Red),
        ("green", Color::Green),
        ("yellow", Color::Yellow),
        ("blue", Color::Blue),
        ("magenta", Color::Magenta),
        ("cyan", Color::Cyan),
        ("gray", Color::Gray),
        ("grey", Color::Gray),
        ("darkgray", Color::DarkGray),
        ("darkgrey", Color::DarkGray),
        ("lightred", Color::LightRed),
        ("lightgreen", Color::LightGreen),
        ("lightyellow", Color::LightYellow),
        ("lightblue", Color::LightBlue),
        ("lightmagenta", Color::LightMagenta),
        ("lightcyan", Color::LightCyan),
        ("white", Color::White),
        ("reset", Color::Reset),
    ];

    for (name, expected_color) in named {
        assert_eq!(parse_hex_color(name).unwrap(), expected_color);
        let upper = name.to_uppercase();
        assert_eq!(parse_hex_color(&upper).unwrap(), expected_color);
    }
}

#[test]
fn test_stress_theme_colors_invalid_hex_strings_never_panic() {
    let invalid_colors = [
        "",                  // Empty
        "   ",               // Whitespace
        "#",                 // Hash only
        "#1",                // 1 hex digit
        "#12",               // 2 hex digits
        "#1234",             // 4 hex digits
        "#12345",            // 5 hex digits
        "#1234567",          // 7 hex digits
        "#12345678",         // 8 hex digits (RGBA)
        "#gggggg",           // Invalid hex chars in 6-digit
        "#12z456",           // Invalid hex char in middle
        "#ggg",              // Invalid hex chars in 3-digit
        "#1z3",              // Invalid hex char in 3-digit
        "1",                 // Bare 1 char
        "12",                // Bare 2 chars
        "1234",              // Bare 4 chars
        "12345",             // Bare 5 chars
        "1234567",           // Bare 7 chars
        "not_a_color",       // Unknown name
        "rgb(255,0,0)",      // CSS rgb syntax
        "hsl(0, 100%, 50%)", // CSS hsl syntax
    ];

    for invalid in invalid_colors {
        let res = parse_hex_color(invalid);
        assert!(
            res.is_err(),
            "Expected parse_hex_color to return Err for invalid string '{}', got {:?}",
            invalid,
            res
        );
    }
}

/// Adversarial stress test: UTF-8 multibyte strings with hash prefix
/// Tests whether slicing `&raw[0..2]` on 6-byte multibyte strings panics with
/// "byte index is not a char boundary".
#[test]
fn test_adversarial_theme_colors_multibyte_utf8_panic_check() {
    // A 4-byte UTF-8 emoji ('🔥' is 4 bytes) + 2 ASCII digits ('1', '2') = 6 bytes in raw!
    // If parse_hex_color slices raw[0..2], it panics because byte 2 is inside '🔥'!
    let emoji_6bytes = "#🔥12";
    let caught_emoji = panic::catch_unwind(|| {
        let _ = parse_hex_color(emoji_6bytes);
    });

    // Three 2-byte UTF-8 chars ('ééé' = 6 bytes in raw)
    // raw[0..2] is boundary, but raw[2..4] and raw[4..6] test internal boundaries
    let multi_2byte = "#ééé";
    let caught_multi = panic::catch_unwind(|| {
        let _ = parse_hex_color(multi_2byte);
    });

    // A 2-byte char ('ñ') + 1 ASCII digit ('1') = 3 bytes in raw!
    // raw[0..1] slices right inside 'ñ'!
    let shorthand_2byte = "#ñ1";
    let caught_shorthand = panic::catch_unwind(|| {
        let _ = parse_hex_color(shorthand_2byte);
    });

    // Check if any panicked:
    let emoji_panicked = caught_emoji.is_err();
    let multi_panicked = caught_multi.is_err();
    let shorthand_panicked = caught_shorthand.is_err();

    println!(
        "Multibyte UTF-8 Panic Probe: emoji_panicked={}, multi_panicked={}, shorthand_panicked={}",
        emoji_panicked, multi_panicked, shorthand_panicked
    );

    // Document whether a panic occurred
    if emoji_panicked || shorthand_panicked {
        eprintln!(
            "VULNERABILITY DETECTED: parse_hex_color panics on multibyte UTF-8 strings matching byte length 6 or 3 (#🔥12 / #ñ1)!"
        );
    }
}

#[test]
fn test_stress_theme_size_and_badge_helpers() {
    let colors = ThemeColors::default();

    // Size threshold checks:
    // Heavy: >= 500 MiB (524_288_000)
    // Medium: >= 50 MiB (52_428_800)
    // Light: < 50 MiB or None
    let style_heavy = colors.size_style_for_bytes(Some(500 * 1024 * 1024));
    assert_eq!(style_heavy.fg, Some(Color::Rgb(0xea, 0x69, 0x62)));

    let style_heavy_plus = colors.size_style_for_bytes(Some(10 * 1024 * 1024 * 1024));
    assert_eq!(style_heavy_plus.fg, Some(Color::Rgb(0xea, 0x69, 0x62)));

    let style_medium = colors.size_style_for_bytes(Some(50 * 1024 * 1024));
    assert_eq!(style_medium.fg, Some(Color::Rgb(0xe7, 0x8a, 0x4e)));

    let style_just_below_med = colors.size_style_for_bytes(Some(50 * 1024 * 1024 - 1));
    assert_eq!(style_just_below_med.fg, Some(Color::Rgb(0xa9, 0xb6, 0x65)));

    let style_zero = colors.size_style_for_bytes(Some(0));
    assert_eq!(style_zero.fg, Some(Color::Rgb(0xa9, 0xb6, 0x65)));

    let style_none = colors.size_style_for_bytes(None);
    assert_eq!(style_none.fg, Some(Color::Rgb(0xa9, 0xb6, 0x65)));

    // Badge helpers:
    assert_eq!(
        colors.badge_style_for_manager("pacman").fg,
        Some(Color::Rgb(0x7d, 0xae, 0xa3))
    );
    assert_eq!(
        colors.badge_style_for_manager("PACMAN").fg,
        Some(Color::Rgb(0x7d, 0xae, 0xa3))
    );
    assert_eq!(
        colors.badge_style_for_manager("aur").fg,
        Some(Color::Rgb(0xe7, 0x8a, 0x4e))
    );
    assert_eq!(
        colors.badge_style_for_manager("yay").fg,
        Some(Color::Rgb(0xe7, 0x8a, 0x4e))
    );
    assert_eq!(
        colors.badge_style_for_manager("flatpak").fg,
        Some(Color::Rgb(0xd3, 0x86, 0x9b))
    );
    assert_eq!(
        colors.badge_style_for_manager("cargo").fg,
        Some(Color::Rgb(0xea, 0x69, 0x62))
    );
    assert_eq!(
        colors.badge_style_for_manager("python").fg,
        Some(Color::Rgb(0xd8, 0xa6, 0x57))
    );
    assert_eq!(
        colors.badge_style_for_manager("uv").fg,
        Some(Color::Rgb(0xd8, 0xa6, 0x57))
    );
    assert_eq!(
        colors.badge_style_for_manager("pip").fg,
        Some(Color::Rgb(0xd8, 0xa6, 0x57))
    );
    // Unknown manager fallback
    assert_eq!(
        colors.badge_style_for_manager("unknown_mgr").fg,
        Some(Color::Rgb(0xd4, 0xbe, 0x98))
    );
}

// =============================================================================
// AREA 4: CustomManagerConfig per MANAGERS_SPEC.md
// =============================================================================

#[test]
fn test_stress_all_spec_managers_from_spec_doc() {
    let spec_toml = r##"
        # --- Gestor de Herramientas Python: uv ---
        [[manager]]
        id = "uv"
        name = "UV Tools"
        badge = "[UV ]"
        color = "#d8a657"
        binary = "uv"
        list_cmd = "uv tool list"
        format = "line_regex"
        parse_regex = '^(?P<name>[a-zA-Z0-9_\-]+)\s+v(?P<version>[0-9\.]+)\s+\((?P<path>.*)\)$'
        size_cmd = "du -sb {path}"
        info_cmd = "uv tool show {name}"
        uninstall_cmd = "uv tool uninstall {name}"
        need_sudo = false

        # --- Gestor de Paquetes Ruby: Gem ---
        [[manager]]
        id = "gem"
        name = "Ruby Gems"
        badge = "[GEM]"
        color = "#ea6962"
        binary = "gem"
        list_cmd = "gem list --local"
        format = "line_regex"
        parse_regex = '^(?P<name>[a-zA-Z0-9_\-]+)\s+\((?P<version>[0-9\., ]+)\)'
        uninstall_cmd = "gem uninstall {name}"
        need_sudo = false

        # --- Homebrew en Linux (Linuxbrew) ---
        [[manager]]
        id = "brew"
        name = "Homebrew"
        badge = "[BRW]"
        color = "#e78a4e"
        binary = "brew"
        list_cmd = "brew info --installed --json=v2"
        format = "json"
        json_array_path = "/formulae"
        json_name       = "/name"
        json_version    = "/versions/stable"
        json_size       = "/installed/0/installed_as_dependency"
        json_desc       = "/desc"
        uninstall_cmd   = "brew uninstall {name}"
        need_sudo       = false

        # --- NPM Global Packages ---
        [[manager]]
        id = "npm"
        name = "NPM Global"
        badge = "[NPM]"
        color = "#ea6962"
        binary = "npm"
        list_cmd = "npm list -g --depth=0 --json"
        format = "json"
        json_map_path   = "/dependencies"
        json_version    = "/version"
        uninstall_cmd   = "npm uninstall -g {name}"
        need_sudo       = false
    "##;

    let managers = parse_managers_toml(spec_toml).expect("All 4 MANAGERS_SPEC examples must parse");
    assert_eq!(managers.len(), 4);

    // Verify UV
    assert_eq!(managers[0].id, "uv");
    assert_eq!(managers[0].name, "UV Tools");
    assert_eq!(managers[0].badge, "[UV ]");
    assert_eq!(managers[0].format, OutputFormat::LineRegex);
    assert_eq!(managers[0].size_cmd.as_deref(), Some("du -sb {path}"));
    assert!(!managers[0].need_sudo);

    // Verify Gem
    assert_eq!(managers[1].id, "gem");
    assert_eq!(managers[1].format, OutputFormat::LineRegex);

    // Verify Brew
    assert_eq!(managers[2].id, "brew");
    assert_eq!(managers[2].format, OutputFormat::Json);
    assert_eq!(managers[2].json_array_path.as_deref(), Some("/formulae"));
    assert_eq!(managers[2].json_name.as_deref(), Some("/name"));

    // Verify NPM
    assert_eq!(managers[3].id, "npm");
    assert_eq!(managers[3].format, OutputFormat::Json);
    assert_eq!(managers[3].json_map_path.as_deref(), Some("/dependencies"));
}

#[test]
fn test_stress_custom_manager_formats_and_defaults() {
    // 1. TSV and CSV formats
    let toml_tsv_csv = r#"
        [[manager]]
        id = "custom_tsv"
        name = "TSV Manager"
        badge = "[TSV]"
        binary = "my_tool"
        list_cmd = "my_tool --tsv"
        format = "tsv"
        uninstall_cmd = "my_tool remove {name}"

        [[manager]]
        id = "custom_csv"
        name = "CSV Manager"
        badge = "[CSV]"
        binary = "my_tool2"
        list_cmd = "my_tool2 --csv"
        format = "csv"
        uninstall_cmd = "my_tool2 remove {name}"
        need_sudo = true
    "#;

    let mgrs = parse_managers_toml(toml_tsv_csv).expect("TSV and CSV formats must parse");
    assert_eq!(mgrs.len(), 2);
    assert_eq!(mgrs[0].format, OutputFormat::Tsv);
    assert!(!mgrs[0].need_sudo); // default false
    assert_eq!(mgrs[0].color, None); // default None
    assert_eq!(mgrs[1].format, OutputFormat::Csv);
    assert!(mgrs[1].need_sudo); // explicit true

    // 2. Empty managers config
    let empty_mgrs = parse_managers_toml("").expect("Empty managers TOML should parse");
    assert!(empty_mgrs.is_empty());
}

#[test]
fn test_stress_custom_manager_malformed_and_missing_fields() {
    // Missing required field 'id'
    let missing_id = r#"
        [[manager]]
        name = "No ID"
        badge = "[NO]"
        binary = "cmd"
        list_cmd = "cmd list"
        format = "line_regex"
        uninstall_cmd = "cmd rm {name}"
    "#;
    assert!(parse_managers_toml(missing_id).is_err());

    // Missing required field 'format'
    let missing_format = r#"
        [[manager]]
        id = "no_format"
        name = "No Format"
        badge = "[NO]"
        binary = "cmd"
        list_cmd = "cmd list"
        uninstall_cmd = "cmd rm {name}"
    "#;
    assert!(parse_managers_toml(missing_format).is_err());

    // Invalid format value
    let invalid_format = r#"
        [[manager]]
        id = "bad_format"
        name = "Bad Format"
        badge = "[BAD]"
        binary = "cmd"
        list_cmd = "cmd list"
        format = "xml_format"
        uninstall_cmd = "cmd rm {name}"
    "#;
    assert!(parse_managers_toml(invalid_format).is_err());
}
