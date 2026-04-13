#![expect(
    unreachable_pub,
    reason = "Configuration resolution is shared between sibling modules in this binary crate."
)]

use anyhow::Result;
use app_config::{DlpConfig, load_dlp_config};

use crate::args::Args;

pub fn resolve_config(args: &Args) -> Result<DlpConfig> {
    let mut config = load_dlp_config()?;

    if let Some(api_scheme) = &args.api_scheme {
        config.api.scheme.clone_from(api_scheme);
    }
    if let Some(api_host) = &args.api_host {
        config.api.host.clone_from(api_host);
    }
    if let Some(api_port) = args.api_port {
        config.api.port = api_port;
    }

    Ok(config)
}
