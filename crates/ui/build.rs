use std::{env, error::Error};

use app_config::load_ui_config_from_dir;

fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let config_path = std::path::Path::new(&manifest_dir)
        .ancestors()
        .nth(2)
        .map(|dir| dir.join("config.toml"))
        .ok_or("workspace root not found")?;

    println!("cargo:rerun-if-changed={}", config_path.display());
    println!("cargo:rerun-if-env-changed=DLP_UI_API_SCHEME");
    println!("cargo:rerun-if-env-changed=DLP_UI_API_HOST");
    println!("cargo:rerun-if-env-changed=DLP_UI_API_PORT");
    println!("cargo:rerun-if-env-changed=DLP_CONFIG_PATH");

    let config = load_ui_config_from_dir(std::path::Path::new(&manifest_dir))?;
    println!(
        "cargo:rustc-env=DLP_UI_API_BASE_URL={}",
        config.api.base_url()
    );

    Ok(())
}
