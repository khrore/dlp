//! Thin executable wrapper around the `dlp` library crate.

use anyhow::Result;
use app_config as _;
#[cfg(test)]
use control_plane as _;
use dlp::{Args, execute_command, resolve_config, run_repl};
use dlp_api as _;
use dlp_client::DlpClient;
use tokio::io::{self, AsyncWriteExt as _};

#[tokio::main]
async fn main() -> Result<()> {
    let args = <Args as clap::Parser>::parse();
    let command = args.command.clone();
    let client = DlpClient::new(resolve_config(&args)?.api.base_url());

    match command {
        Some(parsed_command) => {
            let output = execute_command(parsed_command, &client).await?;
            let mut stdout = io::stdout();
            stdout.write_all(output.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
        }
        None => run_repl(client).await?,
    }

    Ok(())
}
