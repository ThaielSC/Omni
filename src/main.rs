use clap::Parser;
use omni::config::{OmniConfig, default_managers_path};
use omni::model::{SortMode, UnifiedPackage};
use omni::provider::{
    PackageManagerProvider, cargo::CargoProvider, custom::CustomManager, flatpak::FlatpakProvider,
    pacman::PacmanProvider, pip::PipProvider, uv::UvProvider, yay::YayProvider,
};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(name = "omni", version, about = "Unified package inventory for Linux")]
struct Args {
    /// List installed packages (headless mode).
    #[arg(long, short = 'l')]
    list: bool,
    /// Print package list as JSON.
    #[arg(long)]
    json: bool,
    /// Restrict results to one package manager.
    #[arg(long)]
    manager: Option<String>,
    /// Sort results by size, name, or date.
    #[arg(long)]
    sort: Option<String>,
    /// Load an explicit Omni config file.
    #[arg(long)]
    config: Option<PathBuf>,
}

fn execute() -> Result<(), String> {
    let args = Args::parse();
    let config = if let Some(path) = args.config.as_deref() {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read config {}: {e}", path.display()))?;
        OmniConfig::from_toml_str(&contents).map_err(|e| format!("invalid config: {e}"))?
    } else {
        OmniConfig::load_or_default()
    };

    let mut providers: Vec<Box<dyn PackageManagerProvider>> = vec![
        Box::new(PacmanProvider),
        Box::new(YayProvider),
        Box::new(FlatpakProvider),
        Box::new(CargoProvider),
        Box::new(UvProvider),
        Box::new(PipProvider),
    ];
    let mut custom_configs = config.custom_managers;
    if let Some(path) = args
        .config
        .as_deref()
        .and_then(|p| p.parent())
        .map(|p| p.join("managers.toml"))
    {
        if let Ok(mut extra) = omni::config::load_managers_from_path(&path) {
            custom_configs.append(&mut extra);
        }
    } else if let Some(path) = default_managers_path() {
        if let Ok(mut extra) = omni::config::load_managers_from_path(&path) {
            custom_configs.append(&mut extra);
        }
    }
    providers.extend(
        CustomManager::from_configs(custom_configs)
            .into_iter()
            .map(|p| Box::new(p) as Box<dyn PackageManagerProvider>),
    );

    let filter = args
        .manager
        .as_deref()
        .unwrap_or("all")
        .trim()
        .to_ascii_lowercase();
    if args.manager.as_deref().is_some_and(|m| m.trim().is_empty()) {
        return Err("--manager must not be empty".into());
    }
    let sort = match args.sort.as_deref() {
        Some(value) => Some(value.parse::<SortMode>().map_err(|e| e.to_string())?),
        None => None,
    };
    let mut packages: Vec<UnifiedPackage> = Vec::new();
    for provider in providers {
        let id = provider.id().to_ascii_lowercase();
        if filter != "all" && !manager_matches(&filter, &id) {
            continue;
        }
        if !provider.is_available() {
            continue;
        }
        if let Ok(mut found) = provider.list_packages() {
            packages.append(&mut found);
        }
    }
    if let Some(sort) = sort {
        sort.sort_packages(&mut packages);
    } else {
        packages.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.source.name().cmp(b.source.name()))
        });
    }
    if args.json {
        println!(
            "{}",
            serde_json::to_string(&packages).map_err(|e| e.to_string())?
        );
    } else {
        if packages.is_empty() {
            println!("No packages found.");
        } else {
            println!(
                "{:<32} {:<18} {:<10} {:>12}",
                "NAME", "VERSION", "SOURCE", "SIZE"
            );
            for p in packages {
                println!(
                    "{:<32} {:<18} {:<10} {:>12}",
                    p.name,
                    p.version,
                    p.source.name(),
                    omni::utils::format::format_bytes(p.size_bytes)
                );
            }
        }
    }
    Ok(())
}
fn manager_matches(filter: &str, id: &str) -> bool {
    match filter {
        "yay" | "aur" => id == "yay" || id == "aur",
        "python" => id == "uv" || id == "pip",
        other => other == id,
    }
}
fn main() -> ExitCode {
    match std::panic::catch_unwind(execute) {
        Ok(Ok(())) => ExitCode::SUCCESS,
        Ok(Err(error)) => {
            eprintln!("omni: {error}");
            ExitCode::FAILURE
        }
        Err(_) => {
            eprintln!("omni: unexpected internal error");
            ExitCode::FAILURE
        }
    }
}
