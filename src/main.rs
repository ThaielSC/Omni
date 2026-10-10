pub mod config;
pub mod model;
pub mod utils;

fn main() {
    let config = config::OmniConfig::load_or_default();
    println!("Omni v0.1.0 initialized with layout ratio {:?}", config.layout.ratio);
}
