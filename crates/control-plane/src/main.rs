use app_config::load_control_plane_config;
use clap::Parser;
use tokio::net::TcpListener;

#[derive(Debug, Parser)]
#[command(name = "control-plane", about = "DLP control-plane server")]
struct Args {
    #[arg(long)]
    host: Option<std::net::IpAddr>,

    #[arg(long)]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    println!("control-plane listening on http://{address}");
    axum::serve(listener, control_plane::app()).await?;

    Ok(())
}
