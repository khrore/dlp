//! Build script for injecting UI configuration into the WASM frontend.
#![expect(
    clippy::absolute_paths,
    reason = "Using qualified paths keeps the build script dependency-free."
)]
#![expect(
    clippy::missing_docs_in_private_items,
    reason = "The build script has a single local helper."
)]

use std::{
    env,
    error::Error,
    io::{self, Write as _},
    path::Path,
};

use app_config::{find_config_path_from_dir, load_ui_config_from_dir};

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

fn emit_cargo_directive(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_fmt(args)
}
