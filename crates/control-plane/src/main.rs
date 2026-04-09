//! Binary entrypoint for the DLP control-plane server.

use std::{error::Error, net::IpAddr};

use app_config::load_control_plane_config;
use clap::Parser;
use client_sdk as _;
use env_logger as _;
use log::info;
use serde as _;
#[cfg(test)]
use serde_json as _;
use tokio::net::TcpListener;
#[cfg(test)]
use tower as _;

#[derive(Debug, Parser)]
#[command(name = "control-plane", about = "DLP control-plane server")]
struct Args {
    #[arg(long)]
    host: Option<IpAddr>,

    #[arg(long)]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args = Args::parse();
    let mut config = load_control_plane_config()?;
    if let Some(host) = args.host {
        config.server.host = host;
    }
    if let Some(port) = args.port {
        config.server.port = port;
    }

    let address = config.server.socket_addr();
    let listener = TcpListener::bind(address).await?;
    let state = control_plane::new_shared_state();
    control_plane::spawn_reconcile_loop(state.clone());

    info!("control-plane listening on http://{address}");
    axum::serve(listener, control_plane::app(state)).await?;

    Ok(())
}
