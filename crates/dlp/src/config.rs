use anyhow::Result;
use app_config::{DlpConfig, load_dlp_config};

use crate::args::Args;

pub fn resolve_config(args: &Args) -> Result<DlpConfig> {
    let mut config = load_dlp_config()?;

    if let Some(api_scheme) = &args.api_scheme {
        config.api.scheme = api_scheme.clone();
    }
    if let Some(api_host) = &args.api_host {
        config.api.host = api_host.clone();
    }
    if let Some(api_port) = args.api_port {
        config.api.port = api_port;
    }

    Ok(config)
}
