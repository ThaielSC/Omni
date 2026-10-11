use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::Constraint;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::BorderType;
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::path::{Path, PathBuf};

// =============================================================================
// Layout Configuration
// =============================================================================

/// Proportions, panel visibility, and border decorations for the 3-column TUI layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutConfig {
    /// Width ratios for [Managers, Packages, Details]. Default: [1, 2, 3].
    #[serde(deserialize_with = "deserialize_ratio")]
    pub ratio: [usize; 3],
    /// Whether Column 1 (Managers) is shown by default.
    pub show_managers: bool,
    /// Whether Column 3 (Details inspector) is shown by default.
    pub show_details: bool,
    /// Panel decoration: "rounded", "plain", "thick", "double", "vertical", or "none".
    /// `vertical` draws only dividers between panes; `none` draws no lines.
    pub border_style: String,
    /// Shared decoration for every floating dialog. Default: `rounded`.
    /// Accepted styles: rounded, plain, thick, double.
    pub popup_border_style: String,
    /// Inner cell padding. Default: 1.
    pub padding: u16,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            ratio: [1, 2, 3],
            show_managers: true,
            show_details: true,
            border_style: "none".to_string(),
            popup_border_style: "rounded".to_string(),
            padding: 1,
        }
    }
}

fn deserialize_ratio<'de, D>(deserializer: D) -> Result<[usize; 3], D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RatioRepr {
        Exact([usize; 3]),
        Dynamic(Vec<usize>),
    }

    match RatioRepr::deserialize(deserializer)? {
        RatioRepr::Exact(arr) => Ok(arr.map(|val| if val == 0 { 1 } else { val })),
        RatioRepr::Dynamic(vec) => {
            let mut ratio = [1, 2, 3];
            for (i, &val) in vec.iter().take(3).enumerate() {
                ratio[i] = if val == 0 { 1 } else { val };
            }
            Ok(ratio)
        }
    }
}

impl LayoutConfig {
    /// Converts the configured `border_style` string to a `ratatui::widgets::BorderType`.
    pub fn border_type(&self) -> BorderType {
        parse_border_type(&self.border_style, BorderType::Plain)
    }

    /// Shared border style for floating windows (help, confirmations, loading).
    pub fn popup_border_type(&self) -> BorderType {
        parse_border_type(&self.popup_border_style, BorderType::Rounded)
    }

    pub fn has_panel_borders(&self) -> bool {
        matches!(
            self.border_style.to_lowercase().as_str(),
            "rounded" | "plain" | "thick" | "double"
        )
    }

    pub fn has_vertical_separators(&self) -> bool {
        self.border_style.eq_ignore_ascii_case("vertical")
    }

    /// Computes column constraints based on current panel visibility and ratios.
    pub fn column_constraints(&self) -> Vec<Constraint> {
        let mut total = self.ratio[1];
        if self.show_managers {
            total += self.ratio[0];
        }
        if self.show_details {
            total += self.ratio[2];
        }
        let safe_total = if total == 0 { 1 } else { total } as u32;

        let mut constraints = Vec::new();
        if self.show_managers {
            constraints.push(Constraint::Ratio(self.ratio[0] as u32, safe_total));
        }
        constraints.push(Constraint::Ratio(self.ratio[1] as u32, safe_total));
        if self.show_details {
            constraints.push(Constraint::Ratio(self.ratio[2] as u32, safe_total));
        }
        constraints
    }
}

fn parse_border_type(style: &str, fallback: BorderType) -> BorderType {
    match style.to_lowercase().as_str() {
        "rounded" => BorderType::Rounded,
        "plain" => BorderType::Plain,
        "thick" => BorderType::Thick,
        "double" => BorderType::Double,
        _ => fallback,
    }
}

// =============================================================================
// Style Configuration & Color Parsing
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseColorError {
    #[error("Invalid color format: '{0}' (expected #RRGGBB, #RGB, or standard color name)")]
    InvalidFormat(String),
    #[error("Invalid hex digit in '{0}': {1}")]
    InvalidHex(String, String),
}

/// Parses a hex color string or named color into `ratatui::style::Color`.
/// Supports:
/// - "#RRGGBB" or "RRGGBB"
/// - "#RGB" or "RGB" shorthand
/// - Standard ANSI color names ("red", "green", "blue", "reset", etc.)
pub fn parse_hex_color(hex_str: &str) -> Result<Color, ParseColorError> {
    let trimmed = hex_str.trim();
    if trimmed.is_empty() {
        return Err(ParseColorError::InvalidFormat(trimmed.to_string()));
    }

    if let Some(raw) = trimmed.strip_prefix('#') {
        if !raw.is_ascii() || !raw.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ParseColorError::InvalidFormat(trimmed.to_string()));
        }
        if raw.len() == 6 {
            let r = u8::from_str_radix(&raw[0..2], 16)
                .map_err(|e| ParseColorError::InvalidHex(trimmed.to_string(), e.to_string()))?;
            let g = u8::from_str_radix(&raw[2..4], 16)
                .map_err(|e| ParseColorError::InvalidHex(trimmed.to_string(), e.to_string()))?;
            let b = u8::from_str_radix(&raw[4..6], 16)
                .map_err(|e| ParseColorError::InvalidHex(trimmed.to_string(), e.to_string()))?;
            return Ok(Color::Rgb(r, g, b));
        }
        if raw.len() == 3 {
            let r = u8::from_str_radix(&raw[0..1], 16)
                .map_err(|e| ParseColorError::InvalidHex(trimmed.to_string(), e.to_string()))?;
            let g = u8::from_str_radix(&raw[1..2], 16)
                .map_err(|e| ParseColorError::InvalidHex(trimmed.to_string(), e.to_string()))?;
            let b = u8::from_str_radix(&raw[2..3], 16)
                .map_err(|e| ParseColorError::InvalidHex(trimmed.to_string(), e.to_string()))?;
            return Ok(Color::Rgb(r * 17, g * 17, b * 17));
        }
        return Err(ParseColorError::InvalidFormat(trimmed.to_string()));
    }

    // Check named ANSI colors first
    match trimmed.to_lowercase().as_str() {
        "reset" => return Ok(Color::Reset),
        "black" => return Ok(Color::Black),
        "red" => return Ok(Color::Red),
        "green" => return Ok(Color::Green),
        "yellow" => return Ok(Color::Yellow),
        "blue" => return Ok(Color::Blue),
        "magenta" => return Ok(Color::Magenta),
        "cyan" => return Ok(Color::Cyan),
        "gray" | "grey" => return Ok(Color::Gray),
        "darkgray" | "darkgrey" => return Ok(Color::DarkGray),
        "lightred" => return Ok(Color::LightRed),
        "lightgreen" => return Ok(Color::LightGreen),
        "lightyellow" => return Ok(Color::LightYellow),
        "lightblue" => return Ok(Color::LightBlue),
        "lightmagenta" => return Ok(Color::LightMagenta),
        "lightcyan" => return Ok(Color::LightCyan),
        "white" => return Ok(Color::White),
        _ => {}
    }

    // Check non-prefixed hex string
    if trimmed.is_ascii() && trimmed.len() == 6 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        let r = u8::from_str_radix(&trimmed[0..2], 16)
            .map_err(|e| ParseColorError::InvalidHex(trimmed.to_string(), e.to_string()))?;
        let g = u8::from_str_radix(&trimmed[2..4], 16)
            .map_err(|e| ParseColorError::InvalidHex(trimmed.to_string(), e.to_string()))?;
        let b = u8::from_str_radix(&trimmed[4..6], 16)
            .map_err(|e| ParseColorError::InvalidHex(trimmed.to_string(), e.to_string()))?;
        return Ok(Color::Rgb(r, g, b));
    }

    if trimmed.is_ascii() && trimmed.len() == 3 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        let r = u8::from_str_radix(&trimmed[0..1], 16)
            .map_err(|e| ParseColorError::InvalidHex(trimmed.to_string(), e.to_string()))?;
        let g = u8::from_str_radix(&trimmed[1..2], 16)
            .map_err(|e| ParseColorError::InvalidHex(trimmed.to_string(), e.to_string()))?;
        let b = u8::from_str_radix(&trimmed[2..3], 16)
            .map_err(|e| ParseColorError::InvalidHex(trimmed.to_string(), e.to_string()))?;
        return Ok(Color::Rgb(r * 17, g * 17, b * 17));
    }

    Err(ParseColorError::InvalidFormat(trimmed.to_string()))
}

/// Visual style configuration supporting foreground, background, and text modifiers.
/// Deserializes from either a bare string (e.g. `"#7daea3"`) or a full table (`{ fg = "...", bold = true }`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StyleConfig {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
}

impl StyleConfig {
    pub fn new_fg(fg: impl Into<String>) -> Self {
        Self {
            fg: Some(fg.into()),
            ..Default::default()
        }
    }

    pub fn new_fg_bold(fg: impl Into<String>) -> Self {
        Self {
            fg: Some(fg.into()),
            bold: true,
            ..Default::default()
        }
    }

    /// Converts this configuration to a `ratatui::style::Style`.
    pub fn to_style(&self) -> Style {
        let mut style = Style::default();
        if let Some(c) = self.fg.as_deref().and_then(|f| parse_hex_color(f).ok()) {
            style = style.fg(c);
        }
        if let Some(c) = self.bg.as_deref().and_then(|b| parse_hex_color(b).ok()) {
            style = style.bg(c);
        }
        if self.bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        if self.dim {
            style = style.add_modifier(Modifier::DIM);
        }
        if self.italic {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if self.underline {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        style
    }
}

impl Serialize for StyleConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Helper<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            fg: Option<&'a String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            bg: Option<&'a String>,
            #[serde(skip_serializing_if = "std::ops::Not::not")]
            bold: bool,
            #[serde(skip_serializing_if = "std::ops::Not::not")]
            dim: bool,
            #[serde(skip_serializing_if = "std::ops::Not::not")]
            italic: bool,
            #[serde(skip_serializing_if = "std::ops::Not::not")]
            underline: bool,
        }

        Helper {
            fg: self.fg.as_ref(),
            bg: self.bg.as_ref(),
            bold: self.bold,
            dim: self.dim,
            italic: self.italic,
            underline: self.underline,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StyleConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Simple(String),
            Table {
                #[serde(default)]
                fg: Option<String>,
                #[serde(default)]
                bg: Option<String>,
                #[serde(default)]
                bold: bool,
                #[serde(default)]
                dim: bool,
                #[serde(default)]
                italic: bool,
                #[serde(default)]
                underline: bool,
            },
        }

        match Repr::deserialize(deserializer)? {
            Repr::Simple(color) => Ok(StyleConfig {
                fg: Some(color),
                bg: None,
                bold: false,
                dim: false,
                italic: false,
                underline: false,
            }),
            Repr::Table {
                fg,
                bg,
                bold,
                dim,
                italic,
                underline,
            } => Ok(StyleConfig {
                fg,
                bg,
                bold,
                dim,
                italic,
                underline,
            }),
        }
    }
}

// =============================================================================
// Theme Configuration & Gruvbox Palette
// =============================================================================

/// Theme container with theme name and colors table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub name: String,
    pub colors: ThemeColors,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "gruvbox-material".to_string(),
            colors: ThemeColors::default(),
        }
    }
}

/// Full Gruvbox Material palette and UI styling tokens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeColors {
    // Base colors
    pub bg: String,
    pub fg: String,
    pub border: String,
    pub border_focused: String,

    // Optional per-panel backgrounds, defaulting to the global background.
    pub panel_managers_bg: String,
    pub panel_packages_bg: String,
    pub panel_details_bg: String,

    // Selection indicators
    pub selection_bg: String,
    pub selection_fg: String,
    pub cursor_bar: String,

    // Statusline
    pub status_bar_bg: String,
    pub status_bar_fg: String,
    pub mode_normal_bg: String,
    pub mode_normal_fg: String,
    pub mode_search_bg: String,
    pub mode_search_fg: String,
    pub mode_alert_bg: String,
    pub mode_alert_fg: String,

    // Manager Badges
    pub badge_pacman: StyleConfig,
    pub badge_aur: StyleConfig,
    pub badge_flatpak: StyleConfig,
    pub badge_cargo: StyleConfig,
    pub badge_python: StyleConfig,

    // Size Alert Thresholds
    pub size_heavy: StyleConfig,
    pub size_medium: StyleConfig,
    pub size_light: StyleConfig,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            bg: "#282828".to_string(),
            fg: "#d4be98".to_string(),
            border: "#5a524c".to_string(),
            border_focused: "#d8a657".to_string(),

            panel_managers_bg: "#2e2b2a".to_string(),
            panel_packages_bg: "#282828".to_string(),
            panel_details_bg: "#32302f".to_string(),

            selection_bg: "#45403d".to_string(),
            selection_fg: "#d4be98".to_string(),
            cursor_bar: "#d8a657".to_string(),

            status_bar_bg: "#32302f".to_string(),
            status_bar_fg: "#d4be98".to_string(),
            mode_normal_bg: "#a9b665".to_string(),
            mode_normal_fg: "#282828".to_string(),
            mode_search_bg: "#e78a4e".to_string(),
            mode_search_fg: "#282828".to_string(),
            mode_alert_bg: "#ea6962".to_string(),
            mode_alert_fg: "#282828".to_string(),

            badge_pacman: StyleConfig::new_fg_bold("#7daea3"),
            badge_aur: StyleConfig::new_fg_bold("#e78a4e"),
            badge_flatpak: StyleConfig::new_fg_bold("#d3869b"),
            badge_cargo: StyleConfig::new_fg_bold("#ea6962"),
            badge_python: StyleConfig::new_fg_bold("#d8a657"),

            size_heavy: StyleConfig::new_fg("#ea6962"),
            size_medium: StyleConfig::new_fg("#e78a4e"),
            size_light: StyleConfig::new_fg("#a9b665"),
        }
    }
}

impl ThemeColors {
    pub fn bg_color(&self) -> Color {
        parse_hex_color(&self.bg).unwrap_or(Color::Rgb(0x28, 0x28, 0x28))
    }

    pub fn panel_managers_bg_color(&self) -> Color {
        parse_hex_color(&self.panel_managers_bg).unwrap_or_else(|_| self.bg_color())
    }

    pub fn panel_packages_bg_color(&self) -> Color {
        parse_hex_color(&self.panel_packages_bg).unwrap_or_else(|_| self.bg_color())
    }

    pub fn panel_details_bg_color(&self) -> Color {
        parse_hex_color(&self.panel_details_bg).unwrap_or_else(|_| self.bg_color())
    }

    pub fn fg_color(&self) -> Color {
        parse_hex_color(&self.fg).unwrap_or(Color::Rgb(0xd4, 0xbe, 0x98))
    }

    pub fn border_color(&self) -> Color {
        parse_hex_color(&self.border).unwrap_or(Color::Rgb(0x5a, 0x52, 0x4c))
    }

    pub fn border_focused_color(&self) -> Color {
        parse_hex_color(&self.border_focused).unwrap_or(Color::Rgb(0xd8, 0xa6, 0x57))
    }

    pub fn cursor_bar_color(&self) -> Color {
        parse_hex_color(&self.cursor_bar).unwrap_or(Color::Rgb(0xd8, 0xa6, 0x57))
    }

    pub fn selection_style(&self) -> Style {
        Style::default()
            .bg(parse_hex_color(&self.selection_bg).unwrap_or(Color::Rgb(0x45, 0x40, 0x3d)))
            .fg(parse_hex_color(&self.selection_fg).unwrap_or(Color::Rgb(0xd4, 0xbe, 0x98)))
    }

    pub fn status_bar_style(&self) -> Style {
        Style::default()
            .bg(parse_hex_color(&self.status_bar_bg).unwrap_or(Color::Rgb(0x32, 0x30, 0x2f)))
            .fg(parse_hex_color(&self.status_bar_fg).unwrap_or(Color::Rgb(0xd4, 0xbe, 0x98)))
    }

    pub fn mode_normal_style(&self) -> Style {
        Style::default()
            .bg(parse_hex_color(&self.mode_normal_bg).unwrap_or(Color::Rgb(0xa9, 0xb6, 0x65)))
            .fg(parse_hex_color(&self.mode_normal_fg).unwrap_or(Color::Rgb(0x28, 0x28, 0x28)))
            .add_modifier(Modifier::BOLD)
    }

    pub fn mode_search_style(&self) -> Style {
        Style::default()
            .bg(parse_hex_color(&self.mode_search_bg).unwrap_or(Color::Rgb(0xe7, 0x8a, 0x4e)))
            .fg(parse_hex_color(&self.mode_search_fg).unwrap_or(Color::Rgb(0x28, 0x28, 0x28)))
            .add_modifier(Modifier::BOLD)
    }

    pub fn mode_alert_style(&self) -> Style {
        Style::default()
            .bg(parse_hex_color(&self.mode_alert_bg).unwrap_or(Color::Rgb(0xea, 0x69, 0x62)))
            .fg(parse_hex_color(&self.mode_alert_fg).unwrap_or(Color::Rgb(0x28, 0x28, 0x28)))
            .add_modifier(Modifier::BOLD)
    }

    /// Returns the badge style for a given package manager key.
    pub fn badge_style_for_manager(&self, manager: &str) -> Style {
        match manager.to_lowercase().as_str() {
            "pacman" => self.badge_pacman.to_style(),
            "aur" | "yay" => self.badge_aur.to_style(),
            "flatpak" => self.badge_flatpak.to_style(),
            "cargo" => self.badge_cargo.to_style(),
            "python" | "uv" | "pip" => self.badge_python.to_style(),
            _ => Style::default().fg(self.fg_color()),
        }
    }

    /// Returns the size threshold style according to byte count:
    /// - Heavy: >= 500 MiB (524,288,000 bytes)
    /// - Medium: >= 50 MiB (52,428,800 bytes)
    /// - Light: < 50 MiB or unknown
    pub fn size_style_for_bytes(&self, bytes: Option<u64>) -> Style {
        const FIVE_HUNDRED_MIB: u64 = 500 * 1024 * 1024;
        const FIFTY_MIB: u64 = 50 * 1024 * 1024;

        match bytes {
            Some(b) if b >= FIVE_HUNDRED_MIB => self.size_heavy.to_style(),
            Some(b) if b >= FIFTY_MIB => self.size_medium.to_style(),
            _ => self.size_light.to_style(),
        }
    }
}

// =============================================================================
// Keymap Configuration & Event Dispatch
// =============================================================================

/// All rebindable user actions in Omni following Vim/Helix navigation philosophy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    // Spatial Column Navigation
    FocusLeft,
    FocusRight,
    FocusNext,
    FocusPrev,

    // List Navigation
    Down,
    Up,
    PageDown,
    PageUp,
    Top,
    Bottom,

    // Direct Manager Tabs
    GotoAll,
    GotoPacman,
    GotoAur,
    GotoFlatpak,
    GotoCargo,
    GotoPython,

    // Search and Sort
    Search,
    ClearSearch,
    SortToggle,
    ToggleUpdates,
    TogglePackageSelection,
    ClearPackageSelection,

    // Operations
    Uninstall,
    UpdatePackages,
    AddProjectPackage,
    Reload,
    Help,
    Quit,

    // Panes
    ToggleManagers,
    ToggleDetails,
    ToggleDependencies,
    TogglePackageDependencies,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KeyParseError {
    #[error("Empty key notation string")]
    Empty,
    #[error("Invalid key notation syntax: '{0}' (expected '<...>' or single key)")]
    InvalidSyntax(String),
    #[error("Unknown modifier '{0}' in key notation '{1}'")]
    UnknownModifier(String, String),
    #[error("Unknown key code '{0}' in key notation '{1}'")]
    UnknownKeyCode(String, String),
}

/// A list of key combinations bound to an action.
/// Deserializes from either a single string (e.g. "q") or an array of strings (e.g. ["q", "<C-c>"]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KeyBindings(pub Vec<String>);

impl KeyBindings {
    pub fn new<I, S>(bindings: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self(bindings.into_iter().map(Into::into).collect())
    }

    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    pub fn format_summary(&self) -> String {
        self.0.join(", ")
    }

    /// Checks if an incoming crossterm `KeyEvent` matches any of the configured bindings.
    pub fn matches(&self, actual: &KeyEvent) -> bool {
        self.0.iter().any(|notation| {
            if let Ok(expected) = parse_key_event(notation) {
                key_matches(&expected, actual)
            } else {
                false
            }
        })
    }

    /// Parses all configured key notation strings into `KeyEvent` instances.
    pub fn parse_all(&self) -> Result<Vec<KeyEvent>, KeyParseError> {
        self.0.iter().map(|s| parse_key_event(s)).collect()
    }
}

impl<S: Into<String>> From<S> for KeyBindings {
    fn from(s: S) -> Self {
        Self(vec![s.into()])
    }
}

impl<'de> Deserialize<'de> for KeyBindings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeyBindingsVisitor;

        impl<'de> Visitor<'de> for KeyBindingsVisitor {
            type Value = KeyBindings;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(
                    "a key string (e.g. \"q\") or a list of key strings (e.g. [\"q\", \"<C-c>\"])",
                )
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(KeyBindings(vec![value.to_string()]))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(KeyBindings(vec![value]))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(elem) = seq.next_element::<String>()? {
                    vec.push(elem);
                }
                Ok(KeyBindings(vec))
            }
        }

        deserializer.deserialize_any(KeyBindingsVisitor)
    }
}

/// Parses a string key notation (e.g. `"<Left>"`, `"<C-c>"`, `"<Tab>"`, `"j"`, `"d"`, `"1"`)
/// into a crossterm `KeyEvent`.
pub fn parse_key_event(raw: &str) -> Result<KeyEvent, KeyParseError> {
    if raw == " " {
        return Ok(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    }

    let s = raw.trim();
    if s.is_empty() {
        return Err(KeyParseError::Empty);
    }

    if s.starts_with('<') && s.ends_with('>') && s.len() >= 3 {
        let inner = &s[1..s.len() - 1];
        parse_inner_key(inner, s)
    } else if s.len() > 1 && (s.contains('-') || s.contains('+')) {
        parse_inner_key(s, s)
    } else if s.chars().count() == 1 {
        let c = s.chars().next().unwrap();
        let mut modifiers = KeyModifiers::NONE;
        if c.is_ascii_uppercase() {
            modifiers.insert(KeyModifiers::SHIFT);
        }
        Ok(KeyEvent::new(KeyCode::Char(c), modifiers))
    } else {
        parse_named_or_char_key(s, KeyModifiers::NONE, s)
    }
}

fn parse_inner_key(inner: &str, original: &str) -> Result<KeyEvent, KeyParseError> {
    if inner.is_empty() {
        return Err(KeyParseError::InvalidSyntax(original.to_string()));
    }

    let (mod_part, key_part) = if (inner.ends_with('-') || inner.ends_with('+')) && inner.len() > 1
    {
        let last_char = &inner[inner.len() - 1..];
        let prev_char = inner.chars().nth(inner.len() - 2);
        if prev_char == Some('-') || prev_char == Some('+') {
            let m = &inner[..inner.len() - 2];
            (m, last_char)
        } else {
            ("", inner)
        }
    } else if let Some(idx) = inner.rfind(['-', '+']) {
        (&inner[..idx], &inner[idx + 1..])
    } else {
        ("", inner)
    };

    let mut modifiers = KeyModifiers::NONE;
    if !mod_part.is_empty() {
        for token in mod_part.split(['-', '+']) {
            let token_clean = token.trim();
            if token_clean.is_empty() {
                continue;
            }
            match token_clean.to_ascii_lowercase().as_str() {
                "c" | "ctrl" | "control" => modifiers.insert(KeyModifiers::CONTROL),
                "a" | "alt" | "m" | "meta" => modifiers.insert(KeyModifiers::ALT),
                "s" | "shift" => modifiers.insert(KeyModifiers::SHIFT),
                "super" | "cmd" | "command" | "win" | "windows" | "d" => {
                    modifiers.insert(KeyModifiers::SUPER)
                }
                _ => {
                    return Err(KeyParseError::UnknownModifier(
                        token_clean.to_string(),
                        original.to_string(),
                    ));
                }
            }
        }
    }

    parse_named_or_char_key(key_part, modifiers, original)
}

fn parse_named_or_char_key(
    key_part: &str,
    mut modifiers: KeyModifiers,
    original: &str,
) -> Result<KeyEvent, KeyParseError> {
    let lower = key_part.to_ascii_lowercase();
    let code = match lower.as_str() {
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "page_up" | "pgup" => KeyCode::PageUp,
        "pagedown" | "page_down" | "pgdn" | "pgdown" => KeyCode::PageDown,
        "tab" => {
            if modifiers.contains(KeyModifiers::SHIFT) {
                KeyCode::BackTab
            } else {
                KeyCode::Tab
            }
        }
        "backtab" | "back_tab" => KeyCode::BackTab,
        "esc" | "escape" => KeyCode::Esc,
        "enter" | "return" | "ret" => KeyCode::Enter,
        "backspace" | "bs" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "insert" | "ins" => KeyCode::Insert,
        "space" | "spacebar" | "spc" => KeyCode::Char(' '),
        "null" => KeyCode::Null,
        _ => {
            if lower.starts_with('f')
                && lower.len() >= 2
                && let Ok(n @ 1..=12) = lower[1..].parse::<u8>()
            {
                return Ok(KeyEvent::new(KeyCode::F(n), modifiers));
            }

            if key_part.chars().count() == 1 {
                let c = key_part.chars().next().unwrap();
                if c.is_ascii_uppercase() {
                    modifiers.insert(KeyModifiers::SHIFT);
                } else if modifiers.contains(KeyModifiers::SHIFT) {
                    let c_upper = c.to_ascii_uppercase();
                    return Ok(KeyEvent::new(KeyCode::Char(c_upper), modifiers));
                }
                KeyCode::Char(c)
            } else {
                return Err(KeyParseError::UnknownKeyCode(
                    key_part.to_string(),
                    original.to_string(),
                ));
            }
        }
    };

    Ok(KeyEvent::new(code, modifiers))
}

/// Matches an expected key event against an incoming terminal event.
/// Handles case normalization and modifier masks cleanly.
pub fn key_matches(expected: &KeyEvent, actual: &KeyEvent) -> bool {
    if actual.kind == KeyEventKind::Release {
        return false;
    }

    let (exp_code, exp_mods) = normalize_event(expected);
    let (act_code, act_mods) = normalize_event(actual);

    let mask =
        KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT | KeyModifiers::SUPER;
    let exp_filtered = exp_mods & mask;
    let act_filtered = act_mods & mask;

    match (exp_code, act_code) {
        (KeyCode::Char(c1), KeyCode::Char(c2)) => {
            if exp_filtered.contains(KeyModifiers::CONTROL)
                || exp_filtered.contains(KeyModifiers::ALT)
            {
                c1.eq_ignore_ascii_case(&c2) && exp_filtered == act_filtered
            } else {
                c1 == c2 && exp_filtered == act_filtered
            }
        }
        (KeyCode::BackTab, KeyCode::BackTab) => {
            let non_shift_mask = KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER;
            (exp_mods & non_shift_mask) == (act_mods & non_shift_mask)
        }
        (KeyCode::BackTab, KeyCode::Tab) | (KeyCode::Tab, KeyCode::BackTab) => {
            let exp_has_shift =
                exp_mods.contains(KeyModifiers::SHIFT) || exp_code == KeyCode::BackTab;
            let act_has_shift =
                act_mods.contains(KeyModifiers::SHIFT) || act_code == KeyCode::BackTab;
            exp_has_shift && act_has_shift
        }
        (c1, c2) => c1 == c2 && exp_filtered == act_filtered,
    }
}

fn normalize_event(event: &KeyEvent) -> (KeyCode, KeyModifiers) {
    let mut code = event.code;
    let mut mods = event.modifiers;
    if let KeyCode::Char(c) = code {
        if c.is_ascii_uppercase() {
            mods.insert(KeyModifiers::SHIFT);
        } else if mods.contains(KeyModifiers::SHIFT) {
            code = KeyCode::Char(c.to_ascii_uppercase());
        }
    }
    (code, mods)
}

/// Rebindable keymap configuration matching Vim/Helix navigation philosophy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeymapConfig {
    pub focus_left: KeyBindings,
    pub focus_right: KeyBindings,
    pub focus_next: KeyBindings,
    pub focus_prev: KeyBindings,
    pub down: KeyBindings,
    pub up: KeyBindings,
    pub page_down: KeyBindings,
    pub page_up: KeyBindings,
    pub top: KeyBindings,
    pub bottom: KeyBindings,
    pub goto_all: KeyBindings,
    pub goto_pacman: KeyBindings,
    pub goto_aur: KeyBindings,
    pub goto_flatpak: KeyBindings,
    pub goto_cargo: KeyBindings,
    pub goto_python: KeyBindings,
    pub search: KeyBindings,
    pub clear_search: KeyBindings,
    pub sort_toggle: KeyBindings,
    pub toggle_updates: KeyBindings,
    pub toggle_package_selection: KeyBindings,
    pub clear_package_selection: KeyBindings,
    pub uninstall: KeyBindings,
    pub update_packages: KeyBindings,
    pub add_project_package: KeyBindings,
    pub reload: KeyBindings,
    pub help: KeyBindings,
    pub quit: KeyBindings,
    pub toggle_managers: KeyBindings,
    pub toggle_details: KeyBindings,
    pub toggle_dependencies: KeyBindings,
    pub toggle_package_dependencies: KeyBindings,
}

impl Default for KeymapConfig {
    fn default() -> Self {
        Self {
            focus_left: KeyBindings::new(["h", "<Left>"]),
            focus_right: KeyBindings::new(["l", "<Right>"]),
            focus_next: KeyBindings::new(["<Tab>"]),
            focus_prev: KeyBindings::new(["<BackTab>"]),
            down: KeyBindings::new(["j", "<Down>"]),
            up: KeyBindings::new(["k", "<Up>"]),
            page_down: KeyBindings::new(["<C-d>", "<PageDown>"]),
            page_up: KeyBindings::new(["<C-u>", "<PageUp>"]),
            top: KeyBindings::new(["g", "<Home>"]),
            bottom: KeyBindings::new(["G", "<End>"]),
            goto_all: KeyBindings::new(["1"]),
            goto_pacman: KeyBindings::new(["2"]),
            goto_aur: KeyBindings::new(["3"]),
            goto_flatpak: KeyBindings::new(["4"]),
            goto_cargo: KeyBindings::new(["5"]),
            goto_python: KeyBindings::new(["6"]),
            search: KeyBindings::new(["/"]),
            clear_search: KeyBindings::new(["<Esc>"]),
            sort_toggle: KeyBindings::new(["s"]),
            toggle_updates: KeyBindings::new(["u"]),
            toggle_package_selection: KeyBindings::new([" "]),
            clear_package_selection: KeyBindings::new([","]),
            uninstall: KeyBindings::new(["d", "x"]),
            update_packages: KeyBindings::new(["U"]),
            add_project_package: KeyBindings::new(["a"]),
            reload: KeyBindings::new(["r"]),
            help: KeyBindings::new(["?"]),
            quit: KeyBindings::new(["q", "<C-c>"]),
            toggle_managers: KeyBindings::new(["M"]),
            toggle_details: KeyBindings::new(["D"]),
            toggle_dependencies: KeyBindings::new(["e"]),
            toggle_package_dependencies: KeyBindings::new(["i"]),
        }
    }
}

impl KeymapConfig {
    /// Resolves an incoming key event to an `Action` according to configured priority.
    pub fn resolve_action(&self, event: &KeyEvent) -> Option<Action> {
        let action_map = [
            (Action::Quit, &self.quit),
            (Action::Help, &self.help),
            (Action::ClearSearch, &self.clear_search),
            (Action::Search, &self.search),
            (Action::Uninstall, &self.uninstall),
            (Action::UpdatePackages, &self.update_packages),
            (Action::AddProjectPackage, &self.add_project_package),
            (Action::Reload, &self.reload),
            (Action::SortToggle, &self.sort_toggle),
            (Action::ToggleUpdates, &self.toggle_updates),
            (
                Action::TogglePackageSelection,
                &self.toggle_package_selection,
            ),
            (Action::ClearPackageSelection, &self.clear_package_selection),
            (Action::GotoAll, &self.goto_all),
            (Action::GotoPacman, &self.goto_pacman),
            (Action::GotoAur, &self.goto_aur),
            (Action::GotoFlatpak, &self.goto_flatpak),
            (Action::GotoCargo, &self.goto_cargo),
            (Action::GotoPython, &self.goto_python),
            (Action::FocusLeft, &self.focus_left),
            (Action::FocusRight, &self.focus_right),
            (Action::FocusNext, &self.focus_next),
            (Action::FocusPrev, &self.focus_prev),
            (Action::Top, &self.top),
            (Action::Bottom, &self.bottom),
            (Action::PageDown, &self.page_down),
            (Action::PageUp, &self.page_up),
            (Action::Down, &self.down),
            (Action::Up, &self.up),
            (Action::ToggleManagers, &self.toggle_managers),
            (Action::ToggleDetails, &self.toggle_details),
            (Action::ToggleDependencies, &self.toggle_dependencies),
            (
                Action::TogglePackageDependencies,
                &self.toggle_package_dependencies,
            ),
        ];

        for (action, bindings) in action_map {
            if bindings.matches(event) {
                return Some(action);
            }
        }
        None
    }

    /// Checks if a specific action matches the incoming key event.
    pub fn matches(&self, action: Action, event: &KeyEvent) -> bool {
        self.get_bindings(action).matches(event)
    }

    /// Returns a reference to the `KeyBindings` for any given action.
    pub fn get_bindings(&self, action: Action) -> &KeyBindings {
        match action {
            Action::FocusLeft => &self.focus_left,
            Action::FocusRight => &self.focus_right,
            Action::FocusNext => &self.focus_next,
            Action::FocusPrev => &self.focus_prev,
            Action::Down => &self.down,
            Action::Up => &self.up,
            Action::PageDown => &self.page_down,
            Action::PageUp => &self.page_up,
            Action::Top => &self.top,
            Action::Bottom => &self.bottom,
            Action::GotoAll => &self.goto_all,
            Action::GotoPacman => &self.goto_pacman,
            Action::GotoAur => &self.goto_aur,
            Action::GotoFlatpak => &self.goto_flatpak,
            Action::GotoCargo => &self.goto_cargo,
            Action::GotoPython => &self.goto_python,
            Action::Search => &self.search,
            Action::ClearSearch => &self.clear_search,
            Action::SortToggle => &self.sort_toggle,
            Action::ToggleUpdates => &self.toggle_updates,
            Action::TogglePackageSelection => &self.toggle_package_selection,
            Action::ClearPackageSelection => &self.clear_package_selection,
            Action::Uninstall => &self.uninstall,
            Action::UpdatePackages => &self.update_packages,
            Action::AddProjectPackage => &self.add_project_package,
            Action::Reload => &self.reload,
            Action::Help => &self.help,
            Action::Quit => &self.quit,
            Action::ToggleManagers => &self.toggle_managers,
            Action::ToggleDetails => &self.toggle_details,
            Action::ToggleDependencies => &self.toggle_dependencies,
            Action::TogglePackageDependencies => &self.toggle_package_dependencies,
        }
    }
}

// =============================================================================
// Behavior Configuration
// =============================================================================

/// Application behavior options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BehaviorConfig {
    /// Whether to show a confirmation dialog before running uninstall commands.
    pub confirm_uninstall: bool,
    /// Command to elevate privileges (e.g. "sudo", "doas").
    pub sudo_command: String,
    /// Whether to preload deep disk sizes in the background.
    pub preload_disk_sizes: bool,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            confirm_uninstall: true,
            sudo_command: "sudo".to_string(),
            preload_disk_sizes: true,
        }
    }
}

// =============================================================================
// Generic User-Defined Package Managers (docs/MANAGERS_SPEC.md)
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    LineRegex,
    Json,
    Tsv,
    Csv,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomManagerConfig {
    pub id: String,
    pub name: String,
    pub badge: String,
    pub color: Option<String>,
    pub binary: String,
    pub list_cmd: String,
    pub format: OutputFormat,
    pub parse_regex: Option<String>,
    pub json_array_path: Option<String>,
    pub json_map_path: Option<String>,
    pub json_name: Option<String>,
    pub json_version: Option<String>,
    pub json_size: Option<String>,
    pub json_desc: Option<String>,
    pub size_cmd: Option<String>,
    pub info_cmd: Option<String>,
    pub uninstall_cmd: String,
    #[serde(default)]
    pub need_sudo: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ManagersConfig {
    #[serde(rename = "manager", default)]
    pub managers: Vec<CustomManagerConfig>,
}

/// Parses a `managers.toml` content string.
pub fn parse_managers_toml(content: &str) -> Result<Vec<CustomManagerConfig>, toml::de::Error> {
    let cfg: ManagersConfig = toml::from_str(content)?;
    Ok(cfg.managers)
}

/// Loads custom manager configurations from an optional file path.
pub fn load_managers_from_path(
    path: &Path,
) -> Result<Vec<CustomManagerConfig>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let managers = parse_managers_toml(&content)?;
    Ok(managers)
}

// =============================================================================
// Root Application Configuration (OmniConfig) & File Resolution
// =============================================================================

/// Root application configuration aggregating all subsystems.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct OmniConfig {
    pub layout: LayoutConfig,
    pub theme: ThemeConfig,
    pub keymap: KeymapConfig,
    pub behavior: BehaviorConfig,
    pub manager: ManagerConfig,
    #[serde(default)]
    pub custom_managers: Vec<CustomManagerConfig>,
}

/// Configuration for package-manager integrations beyond the global inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ManagerConfig {
    pub projects: ProjectManagersConfig,
}

/// Project-local manifests are discovered from the working directory upward.
/// Each ecosystem can be disabled independently; executable names are
/// configurable for systems that install tools under a non-standard name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectManagersConfig {
    pub enabled: bool,
    pub detect_ancestors: bool,
    pub cargo: bool,
    pub npm: bool,
    pub bun: bool,
    pub cargo_binary: String,
    pub npm_binary: String,
    pub bun_binary: String,
}

impl Default for ProjectManagersConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            detect_ancestors: true,
            cargo: true,
            npm: true,
            bun: true,
            cargo_binary: "cargo".into(),
            npm_binary: "npm".into(),
            bun_binary: "bun".into(),
        }
    }
}

impl ProjectManagersConfig {
    pub fn supports(&self, ecosystem: &str) -> bool {
        self.enabled
            && match ecosystem {
                "cargo" => self.cargo,
                "npm" => self.npm,
                "bun" => self.bun,
                _ => false,
            }
    }
}

/// Returns the standard user configuration directory `~/.config/omni`.
pub fn default_config_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
        && !xdg.is_empty()
    {
        return Some(PathBuf::from(xdg).join("omni"));
    }
    if let Ok(home) = std::env::var("HOME")
        && !home.is_empty()
    {
        return Some(PathBuf::from(home).join(".config").join("omni"));
    }
    None
}

pub fn default_config_path() -> Option<PathBuf> {
    default_config_dir().map(|d| d.join("config.toml"))
}

pub fn default_theme_path() -> Option<PathBuf> {
    default_config_dir().map(|d| d.join("theme.toml"))
}

pub fn default_keymap_path() -> Option<PathBuf> {
    default_config_dir().map(|d| d.join("keymap.toml"))
}

pub fn default_managers_path() -> Option<PathBuf> {
    default_config_dir().map(|d| d.join("managers.toml"))
}

/// Parses a `theme.toml` string supporting both wrapped `[theme]` and root-level `name = ...` forms.
pub fn parse_theme_toml(content: &str) -> Result<ThemeConfig, toml::de::Error> {
    #[derive(Deserialize)]
    struct Wrapper {
        theme: Option<ThemeConfig>,
    }

    if let Ok(Wrapper { theme: Some(t) }) = toml::from_str::<Wrapper>(content) {
        return Ok(t);
    }

    toml::from_str::<ThemeConfig>(content)
}

/// Parses a `keymap.toml` string supporting both wrapped `[keymap]` and root-level bindings.
pub fn parse_keymap_toml(content: &str) -> Result<KeymapConfig, toml::de::Error> {
    #[derive(Deserialize)]
    struct Wrapper {
        keymap: Option<KeymapConfig>,
    }

    if let Ok(Wrapper { keymap: Some(k) }) = toml::from_str::<Wrapper>(content) {
        return Ok(k);
    }

    toml::from_str::<KeymapConfig>(content)
}

impl OmniConfig {
    /// Loads configuration from default paths (~/.config/omni/config.toml, ~/.config/omni/theme.toml, ~/.config/omni/keymap.toml).
    /// If files do not exist or fail to parse, falls back gracefully to defaults.
    pub fn load_or_default() -> Self {
        Self::load_from_paths(
            default_config_path().as_deref(),
            default_theme_path().as_deref(),
            default_keymap_path().as_deref(),
        )
    }

    /// Loads configuration with explicit optional paths for config.toml, theme.toml, and keymap.toml.
    pub fn load_from_paths(
        config_path: Option<&Path>,
        theme_path: Option<&Path>,
        keymap_path: Option<&Path>,
    ) -> Self {
        let mut config = Self::default();

        if let Some(content) = config_path.and_then(|p| std::fs::read_to_string(p).ok())
            && let Ok(parsed) = toml::from_str::<OmniConfig>(&content)
        {
            config = parsed;
        }

        if let Some(content) = theme_path.and_then(|p| std::fs::read_to_string(p).ok())
            && let Ok(theme) = parse_theme_toml(&content)
        {
            config.theme = theme;
        }

        if let Some(content) = keymap_path.and_then(|p| std::fs::read_to_string(p).ok())
            && let Ok(keymap) = parse_keymap_toml(&content)
        {
            config.keymap = keymap;
        }

        // Also check if managers.toml exists in default location
        if let Some(mp) = default_managers_path()
            && let Ok(managers) = load_managers_from_path(&mp)
        {
            config.custom_managers = managers;
        }

        config
    }

    /// Parses configuration from a TOML string (useful for testing or CLI override).
    pub fn from_toml_str(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_layout_config() {
        let layout = LayoutConfig::default();
        assert_eq!(layout.ratio, [1, 2, 3]);
        assert!(layout.show_managers);
        assert!(layout.show_details);
        assert_eq!(layout.border_style, "none");
        assert_eq!(layout.popup_border_style, "rounded");
        assert_eq!(layout.padding, 1);
        assert_eq!(layout.border_type(), BorderType::Plain);
    }

    #[test]
    fn test_layout_border_type_variants() {
        let layout_plain = LayoutConfig {
            border_style: "plain".to_string(),
            ..Default::default()
        };
        assert_eq!(layout_plain.border_type(), BorderType::Plain);

        let layout_thick = LayoutConfig {
            border_style: "thick".to_string(),
            ..Default::default()
        };
        assert_eq!(layout_thick.border_type(), BorderType::Thick);

        let layout_double = LayoutConfig {
            border_style: "double".to_string(),
            ..Default::default()
        };
        assert_eq!(layout_double.border_type(), BorderType::Double);

        let layout_unknown = LayoutConfig {
            border_style: "unknown".to_string(),
            ..Default::default()
        };
        assert_eq!(layout_unknown.border_type(), BorderType::Plain);
    }

    #[test]
    fn test_popup_border_type_variants_and_fallback() {
        for (style, expected) in [
            ("rounded", BorderType::Rounded),
            ("plain", BorderType::Plain),
            ("thick", BorderType::Thick),
            ("double", BorderType::Double),
            ("invalid", BorderType::Rounded),
        ] {
            let layout = LayoutConfig {
                popup_border_style: style.into(),
                ..Default::default()
            };
            assert_eq!(layout.popup_border_type(), expected);
        }
    }

    #[test]
    fn test_layout_constraints_with_visibility_toggles() {
        let layout = LayoutConfig::default();
        assert_eq!(layout.column_constraints().len(), 3);

        let layout_no_mgr = LayoutConfig {
            show_managers: false,
            ..Default::default()
        };
        assert_eq!(layout_no_mgr.column_constraints().len(), 2);

        let layout_pkg_only = LayoutConfig {
            show_managers: false,
            show_details: false,
            ..Default::default()
        };
        assert_eq!(layout_pkg_only.column_constraints().len(), 1);
    }

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(
            parse_hex_color("#282828").unwrap(),
            Color::Rgb(0x28, 0x28, 0x28)
        );
        assert_eq!(
            parse_hex_color("d4be98").unwrap(),
            Color::Rgb(0xd4, 0xbe, 0x98)
        );
        assert_eq!(parse_hex_color("#fff").unwrap(), Color::Rgb(255, 255, 255));
        assert_eq!(parse_hex_color("red").unwrap(), Color::Red);
        assert_eq!(parse_hex_color("green").unwrap(), Color::Green);
        assert_eq!(parse_hex_color("reset").unwrap(), Color::Reset);
        assert!(parse_hex_color("not_a_color").is_err());
        assert!(parse_hex_color("#12").is_err());
    }

    #[test]
    fn test_style_config_deserialization_bare_and_table() {
        let bare: StyleConfig = toml::from_str(r##"fg = "#ea6962""##).unwrap();
        assert_eq!(bare.fg.as_deref(), Some("#ea6962"));
        assert!(!bare.bold);

        let table: StyleConfig = toml::from_str(
            r##"
            fg = "#7daea3"
            bold = true
            "##,
        )
        .unwrap();
        assert_eq!(table.fg.as_deref(), Some("#7daea3"));
        assert!(table.bold);

        let style = table.to_style();
        assert_eq!(style.fg, Some(Color::Rgb(0x7d, 0xae, 0xa3)));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_theme_colors_defaults_and_helpers() {
        let colors = ThemeColors::default();
        assert_eq!(colors.bg_color(), Color::Rgb(0x28, 0x28, 0x28));
        assert_eq!(colors.fg_color(), Color::Rgb(0xd4, 0xbe, 0x98));
        assert_eq!(colors.border_color(), Color::Rgb(0x5a, 0x52, 0x4c));
        assert_eq!(colors.border_focused_color(), Color::Rgb(0xd8, 0xa6, 0x57));

        let pac_style = colors.badge_style_for_manager("pacman");
        assert_eq!(pac_style.fg, Some(Color::Rgb(0x7d, 0xae, 0xa3)));

        let aur_style = colors.badge_style_for_manager("aur");
        assert_eq!(aur_style.fg, Some(Color::Rgb(0xe7, 0x8a, 0x4e)));

        let cargo_style = colors.badge_style_for_manager("cargo");
        assert_eq!(cargo_style.fg, Some(Color::Rgb(0xea, 0x69, 0x62)));

        let heavy = colors.size_style_for_bytes(Some(600 * 1024 * 1024));
        assert_eq!(heavy.fg, Some(Color::Rgb(0xea, 0x69, 0x62)));

        let med = colors.size_style_for_bytes(Some(100 * 1024 * 1024));
        assert_eq!(med.fg, Some(Color::Rgb(0xe7, 0x8a, 0x4e)));

        let light = colors.size_style_for_bytes(Some(10 * 1024 * 1024));
        assert_eq!(light.fg, Some(Color::Rgb(0xa9, 0xb6, 0x65)));
    }

    #[test]
    fn test_parse_key_event() {
        assert_eq!(
            parse_key_event("j").unwrap(),
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)
        );
        assert_eq!(
            parse_key_event("G").unwrap(),
            KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT)
        );
        assert_eq!(
            parse_key_event("<Left>").unwrap(),
            KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)
        );
        assert_eq!(
            parse_key_event("<C-c>").unwrap(),
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
        );
        assert_eq!(
            parse_key_event("<Tab>").unwrap(),
            KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)
        );
        assert_eq!(
            parse_key_event("<BackTab>").unwrap(),
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE)
        );
        assert_eq!(
            parse_key_event("<S-Tab>").unwrap(),
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT)
        );
    }

    #[test]
    fn test_key_matching_logic() {
        let expected_g = parse_key_event("G").unwrap();
        let actual_g_with_shift = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT);
        let actual_g_without_shift = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE);
        assert!(key_matches(&expected_g, &actual_g_with_shift));
        assert!(key_matches(&expected_g, &actual_g_without_shift));

        let expected_ctrl_c = parse_key_event("<C-c>").unwrap();
        let actual_ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(key_matches(&expected_ctrl_c, &actual_ctrl_c));

        let expected_esc = parse_key_event("<Esc>").unwrap();
        let actual_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert!(key_matches(&expected_esc, &actual_esc));

        let release_event =
            KeyEvent::new_with_kind(KeyCode::Esc, KeyModifiers::NONE, KeyEventKind::Release);
        assert!(!key_matches(&expected_esc, &release_event));
    }

    #[test]
    fn test_keymap_config_resolve_action() {
        let keymap = KeymapConfig::default();

        let event_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(keymap.resolve_action(&event_j), Some(Action::Down));

        let event_ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(keymap.resolve_action(&event_ctrl_c), Some(Action::Quit));

        let event_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(keymap.resolve_action(&event_d), Some(Action::Uninstall));

        let event_x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        assert_eq!(keymap.resolve_action(&event_x), Some(Action::Uninstall));

        let event_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(keymap.resolve_action(&event_esc), Some(Action::ClearSearch));
    }

    #[test]
    fn test_omni_config_deserialization_from_toml() {
        let toml_str = r#"
            [layout]
            ratio = [ 2, 3, 4 ]
            border_style = "double"

            [behavior]
            sudo_command = "doas"
            confirm_uninstall = false

            [keymap]
            quit = "<C-q>"
        "#;

        let config = OmniConfig::from_toml_str(toml_str).unwrap();
        assert_eq!(config.layout.ratio, [2, 3, 4]);
        assert_eq!(config.layout.border_style, "double");
        assert_eq!(config.behavior.sudo_command, "doas");
        assert!(!config.behavior.confirm_uninstall);
        assert_eq!(config.keymap.quit.as_slice(), &["<C-q>"]);
        assert_eq!(config.keymap.down.as_slice(), &["j", "<Down>"]);
    }

    #[test]
    fn test_layout_clamped_ratio_deserialization() {
        let toml_str = r#"
            [layout]
            ratio = [ 1, 2 ]
        "#;

        let config = OmniConfig::from_toml_str(toml_str).unwrap();
        assert_eq!(config.layout.ratio, [1, 2, 3]);
    }

    #[test]
    fn test_custom_manager_deserialization() {
        let toml_str = r##"
            [[manager]]
            id = "uv"
            name = "UV Tools"
            badge = "[UV ]"
            color = "#d8a657"
            binary = "uv"
            list_cmd = "uv tool list"
            format = "line_regex"
            parse_regex = '^(?P<name>[a-zA-Z0-9_\-]+)\s+v(?P<version>[0-9\.]+)'
            uninstall_cmd = "uv tool uninstall {name}"
            need_sudo = false
        "##;

        let managers = parse_managers_toml(toml_str).unwrap();
        assert_eq!(managers.len(), 1);
        assert_eq!(managers[0].id, "uv");
        assert_eq!(managers[0].name, "UV Tools");
        assert_eq!(managers[0].format, OutputFormat::LineRegex);
        assert!(!managers[0].need_sudo);
    }

    #[test]
    fn test_parse_hex_color_multibyte_utf8_boundary_safety() {
        // Multi-byte UTF-8 inputs matching byte length 6
        assert_eq!(
            parse_hex_color("#🔥12"),
            Err(ParseColorError::InvalidFormat("#🔥12".to_string()))
        );
        assert_eq!(
            parse_hex_color("#a\u{00e9}bcd"),
            Err(ParseColorError::InvalidFormat("#a\u{00e9}bcd".to_string()))
        );
        assert_eq!(
            parse_hex_color("#ééé"),
            Err(ParseColorError::InvalidFormat("#ééé".to_string()))
        );

        // Multi-byte UTF-8 inputs matching byte length 3
        assert_eq!(
            parse_hex_color("#ñ1"),
            Err(ParseColorError::InvalidFormat("#ñ1".to_string()))
        );
        assert_eq!(
            parse_hex_color("#\u{20ac}"),
            Err(ParseColorError::InvalidFormat("#€".to_string()))
        );

        // ASCII strings with non-hex characters
        assert_eq!(
            parse_hex_color("#gggggg"),
            Err(ParseColorError::InvalidFormat("#gggggg".to_string()))
        );
        assert_eq!(
            parse_hex_color("#ggg"),
            Err(ParseColorError::InvalidFormat("#ggg".to_string()))
        );
        assert_eq!(
            parse_hex_color("#12z456"),
            Err(ParseColorError::InvalidFormat("#12z456".to_string()))
        );

        // Non-prefixed multibyte strings
        assert_eq!(
            parse_hex_color("🔥12"),
            Err(ParseColorError::InvalidFormat("🔥12".to_string()))
        );
        assert_eq!(
            parse_hex_color("ééé"),
            Err(ParseColorError::InvalidFormat("ééé".to_string()))
        );
        assert_eq!(
            parse_hex_color("ñ1"),
            Err(ParseColorError::InvalidFormat("ñ1".to_string()))
        );

        // Valid colors continue to succeed
        assert_eq!(
            parse_hex_color("#282828").unwrap(),
            Color::Rgb(0x28, 0x28, 0x28)
        );
        assert_eq!(parse_hex_color("#fff").unwrap(), Color::Rgb(255, 255, 255));
        assert_eq!(
            parse_hex_color("d4be98").unwrap(),
            Color::Rgb(0xd4, 0xbe, 0x98)
        );
        assert_eq!(parse_hex_color("red").unwrap(), Color::Red);
    }

    #[test]
    fn test_layout_clamped_ratio_exact_and_zeros() {
        let toml_exact_zero = r#"
            [layout]
            ratio = [ 0, 2, 3 ]
        "#;
        let config = OmniConfig::from_toml_str(toml_exact_zero).unwrap();
        assert_eq!(config.layout.ratio, [1, 2, 3]);

        let toml_all_zeros = r#"
            [layout]
            ratio = [ 0, 0, 0 ]
        "#;
        let config2 = OmniConfig::from_toml_str(toml_all_zeros).unwrap();
        assert_eq!(config2.layout.ratio, [1, 1, 1]);

        let toml_middle_zero = r#"
            [layout]
            ratio = [ 3, 0, 5 ]
        "#;
        let config3 = OmniConfig::from_toml_str(toml_middle_zero).unwrap();
        assert_eq!(config3.layout.ratio, [3, 1, 5]);
    }

    #[test]
    fn test_default_keymap_path_resolution() {
        if let Some(path) = default_keymap_path() {
            assert!(path.ends_with("omni/keymap.toml") || path.ends_with("omni\\keymap.toml"));
        }
    }

    #[test]
    fn test_load_from_paths_all_none_returns_defaults() {
        let config = OmniConfig::load_from_paths(None, None, None);
        assert_eq!(config.layout, LayoutConfig::default());
        assert_eq!(config.theme, ThemeConfig::default());
        assert_eq!(config.keymap, KeymapConfig::default());
        assert_eq!(config.behavior, BehaviorConfig::default());
    }

    #[test]
    fn test_load_from_paths_with_keymap_wrapped() {
        use std::io::Write;
        let mut tmp_file = tempfile::NamedTempFile::new().unwrap();
        let content = r#"
            [keymap]
            quit = "<C-x>"
            down = ["j", "s"]
        "#;
        tmp_file.write_all(content.as_bytes()).unwrap();

        let config = OmniConfig::load_from_paths(None, None, Some(tmp_file.path()));
        assert_eq!(config.keymap.quit.as_slice(), &["<C-x>"]);
        assert_eq!(config.keymap.down.as_slice(), &["j", "s"]);
        assert_eq!(config.keymap.up.as_slice(), &["k", "<Up>"]);
        assert_eq!(config.layout, LayoutConfig::default());
    }

    #[test]
    fn test_load_from_paths_with_keymap_bare() {
        use std::io::Write;
        let mut tmp_file = tempfile::NamedTempFile::new().unwrap();
        let content = r#"
            quit = "Z"
            help = "F1"
        "#;
        tmp_file.write_all(content.as_bytes()).unwrap();

        let config = OmniConfig::load_from_paths(None, None, Some(tmp_file.path()));
        assert_eq!(config.keymap.quit.as_slice(), &["Z"]);
        assert_eq!(config.keymap.help.as_slice(), &["F1"]);
        assert_eq!(config.keymap.down.as_slice(), &["j", "<Down>"]);
    }

    #[test]
    fn test_load_from_paths_keymap_overrides_config_toml() {
        use std::io::Write;
        let mut cfg_file = tempfile::NamedTempFile::new().unwrap();
        let cfg_content = r#"
            [layout]
            ratio = [2, 3, 4]

            [keymap]
            quit = "Q"
        "#;
        cfg_file.write_all(cfg_content.as_bytes()).unwrap();

        let mut km_file = tempfile::NamedTempFile::new().unwrap();
        let km_content = r#"
            [keymap]
            quit = "<C-q>"
        "#;
        km_file.write_all(km_content.as_bytes()).unwrap();

        let config = OmniConfig::load_from_paths(Some(cfg_file.path()), None, Some(km_file.path()));

        assert_eq!(config.layout.ratio, [2, 3, 4]);
        assert_eq!(config.keymap.quit.as_slice(), &["<C-q>"]);
    }

    #[test]
    fn test_load_from_paths_missing_or_corrupt_keymap_graceful() {
        let non_existent = PathBuf::from("/non/existent/path/omni/keymap.toml");
        let config_missing = OmniConfig::load_from_paths(None, None, Some(&non_existent));
        assert_eq!(config_missing.keymap, KeymapConfig::default());

        use std::io::Write;
        let mut bad_file = tempfile::NamedTempFile::new().unwrap();
        bad_file.write_all(b"not a valid [[[[ toml").unwrap();
        let config_bad = OmniConfig::load_from_paths(None, None, Some(bad_file.path()));
        assert_eq!(config_bad.keymap, KeymapConfig::default());
    }

    #[test]
    fn test_load_or_default_completes_without_panic() {
        let config = OmniConfig::load_or_default();
        assert_eq!(config.layout.ratio.len(), 3);
        assert!(!config.keymap.quit.as_slice().is_empty());
    }
}
