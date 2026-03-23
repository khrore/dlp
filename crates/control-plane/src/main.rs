use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use clap::Parser;
use tokio::net::TcpListener;

#[derive(Debug, Parser)]
#[command(name = "control-plane", about = "DLP control-plane server")]
struct Args {
    #[arg(long, env = "DLP_SERVER_HOST", default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    host: IpAddr,

    #[arg(long, env = "DLP_SERVER_PORT", default_value_t = 3000)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let address = SocketAddr::from((args.host, args.port));
    let listener = TcpListener::bind(address).await?;

    println!("control-plane listening on http://{address}");
    axum::serve(listener, control_plane::app()).await?;

    Ok(())
}
