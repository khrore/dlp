//! Build script for injecting UI configuration into the WASM frontend.
use std::{
    env,
    error::Error,
    fmt,
    io::{self, Write as _},
    path::Path,
};

use dlp_config::{find_config_path_from_dir, load_ui_config_from_dir};

fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let manifest_path = Path::new(&manifest_dir);
    let maybe_config_path = find_config_path_from_dir(manifest_path);

    if let Some(config_path) = maybe_config_path {
        emit_cargo_directive(format_args!(
            "cargo:rerun-if-changed={}\n",
            config_path.display()
        ))?;
    }
    emit_cargo_directive(format_args!(
        "cargo:rerun-if-env-changed=DLP_UI_API_SCHEME\n"
    ))?;
    emit_cargo_directive(format_args!("cargo:rerun-if-env-changed=DLP_UI_API_HOST\n"))?;
    emit_cargo_directive(format_args!("cargo:rerun-if-env-changed=DLP_UI_API_PORT\n"))?;
    emit_cargo_directive(format_args!("cargo:rerun-if-env-changed=DLP_CONFIG_PATH\n"))?;

    let config = load_ui_config_from_dir(manifest_path)?;
    emit_cargo_directive(format_args!(
        "cargo:rustc-env=DLP_UI_API_BASE_URL={}\n",
        config.api.base_url()
    ))?;

    Ok(())
}

/// Writes one Cargo build-script directive to stdout.
fn emit_cargo_directive(args: fmt::Arguments<'_>) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_fmt(args)
}
