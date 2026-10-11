use serde::{Deserialize, Serialize};

/// Identifies the package manager source of an installed package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageSource {
    Pacman,
    Aur,
    Flatpak,
    Cargo,
    Uv,
    Pip,
}

/// Whether an installed package was installed directly by the user or brought
/// in as another package's dependency. Some package managers do not expose
/// this distinction, in which case the value remains `Unknown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InstallReason {
    Explicit,
    Dependency,
    #[default]
    Unknown,
}

impl PackageSource {
    /// Canonical lowercase identifier for CLI arguments and serialization.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Pacman => "pacman",
            Self::Aur => "aur",
            Self::Flatpak => "flatpak",
            Self::Cargo => "cargo",
            Self::Uv => "uv",
            Self::Pip => "pip",
        }
    }

    /// User-friendly display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Pacman => "Pacman",
            Self::Aur => "AUR",
            Self::Flatpak => "Flatpak",
            Self::Cargo => "Cargo",
            Self::Uv => "uv",
            Self::Pip => "pip",
        }
    }

    /// Short badge label for list rendering.
    pub fn badge_label(&self) -> &'static str {
        match self {
            Self::Pacman => "pacman",
            Self::Aur => "aur",
            Self::Flatpak => "flatpak",
            Self::Cargo => "cargo",
            Self::Uv => "uv",
            Self::Pip => "pip",
        }
    }
}

impl std::fmt::Display for PackageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for PackageSource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "pacman" => Ok(Self::Pacman),
            "aur" | "yay" => Ok(Self::Aur),
            "flatpak" => Ok(Self::Flatpak),
            "cargo" => Ok(Self::Cargo),
            "uv" => Ok(Self::Uv),
            "pip" | "python" => Ok(Self::Pip),
            _ => Err(format!("Unknown package source: '{s}'")),
        }
    }
}

/// Navigation filter tabs in the UI column 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FilterTab {
    #[default]
    All,
    Pacman,
    Aur,
    Flatpak,
    Cargo,
    Python,
}

impl FilterTab {
    pub const ALL: [FilterTab; 6] = [
        FilterTab::All,
        FilterTab::Pacman,
        FilterTab::Aur,
        FilterTab::Flatpak,
        FilterTab::Cargo,
        FilterTab::Python,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Pacman => "Pacman",
            Self::Aur => "AUR",
            Self::Flatpak => "Flatpak",
            Self::Cargo => "Cargo",
            Self::Python => "Python",
        }
    }

    /// Checks if a package source matches this filter tab.
    pub fn matches(&self, source: PackageSource) -> bool {
        match self {
            Self::All => true,
            Self::Pacman => source == PackageSource::Pacman,
            Self::Aur => source == PackageSource::Aur,
            Self::Flatpak => source == PackageSource::Flatpak,
            Self::Cargo => source == PackageSource::Cargo,
            Self::Python => source == PackageSource::Uv || source == PackageSource::Pip,
        }
    }

    /// Tab column index (0..=5).
    pub fn index(&self) -> usize {
        match self {
            Self::All => 0,
            Self::Pacman => 1,
            Self::Aur => 2,
            Self::Flatpak => 3,
            Self::Cargo => 4,
            Self::Python => 5,
        }
    }

    /// Resolve tab from column index.
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::All),
            1 => Some(Self::Pacman),
            2 => Some(Self::Aur),
            3 => Some(Self::Flatpak),
            4 => Some(Self::Cargo),
            5 => Some(Self::Python),
            _ => None,
        }
    }

    /// Resolve tab from keyboard shortcut ('1'..'6').
    pub fn from_number_key(ch: char) -> Option<Self> {
        match ch {
            '1' => Some(Self::All),
            '2' => Some(Self::Pacman),
            '3' => Some(Self::Aur),
            '4' => Some(Self::Flatpak),
            '5' => Some(Self::Cargo),
            '6' => Some(Self::Python),
            _ => None,
        }
    }
}

impl std::fmt::Display for FilterTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Package sorting mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortMode {
    #[default]
    SizeDesc,
    NameAsc,
    DateDesc,
}

impl SortMode {
    /// Cycles to the next sort mode.
    pub fn next(&self) -> Self {
        match self {
            Self::SizeDesc => Self::NameAsc,
            Self::NameAsc => Self::DateDesc,
            Self::DateDesc => Self::SizeDesc,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::SizeDesc => "Size ▼",
            Self::NameAsc => "Name ▲",
            Self::DateDesc => "Date ▼",
        }
    }

    /// Sorts a package slice in-place according to the current mode.
    /// In SizeDesc, all packages with a known size (`Some(bytes)`) are placed before unknown (`None`) sizes.
    pub fn sort_packages(&self, packages: &mut [UnifiedPackage]) {
        match self {
            Self::SizeDesc => {
                packages.sort_by(|a, b| match (a.size_bytes, b.size_bytes) {
                    (Some(sa), Some(sb)) => sb
                        .cmp(&sa)
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                        .then_with(|| a.version.cmp(&b.version)),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a
                        .name
                        .to_lowercase()
                        .cmp(&b.name.to_lowercase())
                        .then_with(|| a.version.cmp(&b.version)),
                });
            }
            Self::NameAsc => {
                packages.sort_by(|a, b| {
                    a.name
                        .to_lowercase()
                        .cmp(&b.name.to_lowercase())
                        .then_with(|| a.version.cmp(&b.version))
                        .then_with(|| a.source.name().cmp(b.source.name()))
                });
            }
            Self::DateDesc => {
                packages.sort_by(|a, b| {
                    b.install_date
                        .as_deref()
                        .unwrap_or("")
                        .cmp(a.install_date.as_deref().unwrap_or(""))
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                        .then_with(|| a.version.cmp(&b.version))
                });
            }
        }
    }
}

impl std::fmt::Display for SortMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for SortMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "size" | "sizedesc" | "size_desc" => Ok(Self::SizeDesc),
            "name" | "nameasc" | "name_asc" => Ok(Self::NameAsc),
            "date" | "datedesc" | "date_desc" => Ok(Self::DateDesc),
            _ => Err(format!("Unknown sort mode: '{s}'")),
        }
    }
}

/// Unified package metadata across all supported package managers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnifiedPackage {
    pub name: String,
    pub version: String,
    pub source: PackageSource,
    /// Original manager identifier for dynamically configured managers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manager_id: Option<String>,
    #[serde(default, skip_serializing_if = "InstallReason::is_unknown")]
    pub install_reason: InstallReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub binaries: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_date: Option<String>,
}

impl UnifiedPackage {
    pub fn new(name: impl Into<String>, version: impl Into<String>, source: PackageSource) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            source,
            manager_id: None,
            install_reason: InstallReason::Unknown,
            size_bytes: None,
            description: None,
            dependencies: Vec::new(),
            files: Vec::new(),
            binaries: Vec::new(),
            url: None,
            license: None,
            install_date: None,
        }
    }

    pub fn with_install_reason(mut self, reason: InstallReason) -> Self {
        self.install_reason = reason;
        self
    }

    pub fn with_size(mut self, size_bytes: u64) -> Self {
        self.size_bytes = Some(size_bytes);
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn with_files(mut self, files: Vec<String>) -> Self {
        self.files = files;
        self
    }

    pub fn with_binaries(mut self, binaries: Vec<String>) -> Self {
        self.binaries = binaries;
        self
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn with_license(mut self, license: impl Into<String>) -> Self {
        self.license = Some(license.into());
        self
    }

    pub fn with_install_date(mut self, date: impl Into<String>) -> Self {
        self.install_date = Some(date.into());
        self
    }

    /// Case-insensitive substring query match on name or description.
    pub fn matches_query(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        if self.name.to_lowercase().contains(&q) {
            return true;
        }
        if let Some(ref desc) = self.description {
            return desc.to_lowercase().contains(&q);
        }
        false
    }
}

impl InstallReason {
    fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

/// Domain errors encountered during package discovery or management.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Manager '{0}' is not available on this system")]
    NotAvailable(String),
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse output: {0}")]
    ParseError(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Events streamed by background discovery providers to the engine.
#[derive(Debug, Clone)]
pub enum ProviderEvent {
    Started(PackageSource),
    PackagesLoaded(PackageSource, Vec<UnifiedPackage>),
    Failed(PackageSource, String),
    Completed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_source_display_and_parse() {
        assert_eq!(PackageSource::Pacman.name(), "pacman");
        assert_eq!(PackageSource::Aur.display_name(), "AUR");
        assert_eq!(PackageSource::Cargo.badge_label(), "cargo");
        assert_eq!(
            "pacman".parse::<PackageSource>().unwrap(),
            PackageSource::Pacman
        );
        assert_eq!("yay".parse::<PackageSource>().unwrap(), PackageSource::Aur);
        assert_eq!("aur".parse::<PackageSource>().unwrap(), PackageSource::Aur);
        assert_eq!(
            "flatpak".parse::<PackageSource>().unwrap(),
            PackageSource::Flatpak
        );
        assert_eq!(
            "cargo".parse::<PackageSource>().unwrap(),
            PackageSource::Cargo
        );
        assert_eq!("uv".parse::<PackageSource>().unwrap(), PackageSource::Uv);
        assert_eq!(
            "python".parse::<PackageSource>().unwrap(),
            PackageSource::Pip
        );
        assert_eq!("pip".parse::<PackageSource>().unwrap(), PackageSource::Pip);
        assert!("unknown_mgr".parse::<PackageSource>().is_err());
    }

    #[test]
    fn test_filter_tab_matching() {
        assert!(FilterTab::All.matches(PackageSource::Pacman));
        assert!(FilterTab::All.matches(PackageSource::Cargo));
        assert!(FilterTab::Pacman.matches(PackageSource::Pacman));
        assert!(!FilterTab::Pacman.matches(PackageSource::Aur));
        assert!(FilterTab::Aur.matches(PackageSource::Aur));
        assert!(FilterTab::Flatpak.matches(PackageSource::Flatpak));
        assert!(FilterTab::Cargo.matches(PackageSource::Cargo));
        assert!(FilterTab::Python.matches(PackageSource::Uv));
        assert!(FilterTab::Python.matches(PackageSource::Pip));
        assert!(!FilterTab::Python.matches(PackageSource::Cargo));
    }

    #[test]
    fn test_filter_tab_indices_and_keys() {
        assert_eq!(FilterTab::from_index(0), Some(FilterTab::All));
        assert_eq!(FilterTab::from_index(1), Some(FilterTab::Pacman));
        assert_eq!(FilterTab::from_index(2), Some(FilterTab::Aur));
        assert_eq!(FilterTab::from_index(3), Some(FilterTab::Flatpak));
        assert_eq!(FilterTab::from_index(4), Some(FilterTab::Cargo));
        assert_eq!(FilterTab::from_index(5), Some(FilterTab::Python));
        assert_eq!(FilterTab::from_index(6), None);

        assert_eq!(FilterTab::from_number_key('1'), Some(FilterTab::All));
        assert_eq!(FilterTab::from_number_key('2'), Some(FilterTab::Pacman));
        assert_eq!(FilterTab::from_number_key('3'), Some(FilterTab::Aur));
        assert_eq!(FilterTab::from_number_key('4'), Some(FilterTab::Flatpak));
        assert_eq!(FilterTab::from_number_key('5'), Some(FilterTab::Cargo));
        assert_eq!(FilterTab::from_number_key('6'), Some(FilterTab::Python));
        assert_eq!(FilterTab::from_number_key('9'), None);
        assert_eq!(FilterTab::from_number_key('a'), None);
    }

    #[test]
    fn test_sort_mode_cycling() {
        assert_eq!(SortMode::SizeDesc.next(), SortMode::NameAsc);
        assert_eq!(SortMode::NameAsc.next(), SortMode::DateDesc);
        assert_eq!(SortMode::DateDesc.next(), SortMode::SizeDesc);

        assert_eq!("size".parse::<SortMode>().unwrap(), SortMode::SizeDesc);
        assert_eq!("name".parse::<SortMode>().unwrap(), SortMode::NameAsc);
        assert_eq!("date".parse::<SortMode>().unwrap(), SortMode::DateDesc);
        assert!("invalid".parse::<SortMode>().is_err());
    }

    #[test]
    fn test_sort_packages_by_size_desc() {
        let mut pkgs = vec![
            UnifiedPackage::new("pkg-small", "1.0", PackageSource::Cargo).with_size(100),
            UnifiedPackage::new("pkg-large", "1.0", PackageSource::Pacman).with_size(1000),
            UnifiedPackage::new("pkg-mid", "1.0", PackageSource::Aur).with_size(500),
            UnifiedPackage::new("pkg-zero", "1.0", PackageSource::Pacman).with_size(0),
            UnifiedPackage::new("pkg-none", "1.0", PackageSource::Flatpak),
        ];

        SortMode::SizeDesc.sort_packages(&mut pkgs);
        assert_eq!(pkgs[0].name, "pkg-large");
        assert_eq!(pkgs[1].name, "pkg-mid");
        assert_eq!(pkgs[2].name, "pkg-small");
        assert_eq!(pkgs[3].name, "pkg-zero");
        assert_eq!(pkgs[4].name, "pkg-none");
    }

    #[test]
    fn test_sort_packages_by_name_asc() {
        let mut pkgs = vec![
            UnifiedPackage::new("zebra", "1.0", PackageSource::Cargo),
            UnifiedPackage::new("apple", "1.0", PackageSource::Pacman),
            UnifiedPackage::new("Banana", "1.0", PackageSource::Aur),
        ];

        SortMode::NameAsc.sort_packages(&mut pkgs);
        assert_eq!(pkgs[0].name, "apple");
        assert_eq!(pkgs[1].name, "Banana");
        assert_eq!(pkgs[2].name, "zebra");
    }

    #[test]
    fn test_sort_packages_by_date_desc() {
        let mut pkgs = vec![
            UnifiedPackage::new("pkg-old", "1.0", PackageSource::Cargo)
                .with_install_date("2025-01-01"),
            UnifiedPackage::new("pkg-new", "1.0", PackageSource::Pacman)
                .with_install_date("2026-09-01"),
            UnifiedPackage::new("pkg-nodate", "1.0", PackageSource::Aur),
        ];

        SortMode::DateDesc.sort_packages(&mut pkgs);
        assert_eq!(pkgs[0].name, "pkg-new");
        assert_eq!(pkgs[1].name, "pkg-old");
        assert_eq!(pkgs[2].name, "pkg-nodate");
    }

    #[test]
    fn test_unified_package_serialization() {
        let pkg = UnifiedPackage::new("ripgrep", "14.1.0", PackageSource::Pacman)
            .with_size(5_000_000)
            .with_description("Fast line-oriented search tool")
            .with_binaries(vec!["rg".to_string()]);

        let json = serde_json::to_string(&pkg).expect("serialize");
        let deserialized: UnifiedPackage = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(pkg, deserialized);
    }

    #[test]
    fn test_unified_package_matches_query() {
        let pkg = UnifiedPackage::new("helix", "25.01", PackageSource::Cargo)
            .with_description("A post-modern modal text editor");

        assert!(pkg.matches_query("helix"));
        assert!(pkg.matches_query("HELIX"));
        assert!(pkg.matches_query("modal"));
        assert!(!pkg.matches_query("neovim"));
    }
}
