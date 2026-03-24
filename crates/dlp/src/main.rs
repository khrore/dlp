use anyhow::{Result, bail};
use app_config::{DlpConfig, load_dlp_config};
use clap::{Parser, Subcommand};
use client_sdk::DlpClient;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Parser)]
#[command(name = "dlp", about = "DLP client with shared CLI and REPL")]
struct Args {
    #[arg(long, global = true)]
    api_scheme: Option<String>,

    #[arg(long, global = true)]
    api_host: Option<String>,

    #[arg(long, global = true)]
    api_port: Option<u16>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
enum Command {
    Health,
}

#[derive(Debug, Clone)]
enum InteractiveCommand {
    Health,
    Help,
    Exit,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let command = args.command.clone();
    let client = DlpClient::new(resolve_config(args)?.api.base_url());

    match command {
        Some(command) => {
            let output = execute_command(command, &client).await?;
            println!("{output}");
        }
        None => run_repl(client).await?,
    }

    Ok(())
}

fn resolve_config(args: Args) -> Result<DlpConfig> {
    let mut config = load_dlp_config()?;

    if let Some(api_scheme) = args.api_scheme {
        config.api.scheme = api_scheme;
    }
    if let Some(api_host) = args.api_host {
        config.api.host = api_host;
    }
    if let Some(api_port) = args.api_port {
        config.api.port = api_port;
    }

    Ok(config)
}

async fn execute_command(command: Command, client: &DlpClient) -> Result<String> {
    match command {
        Command::Health => {
            let health = client.health_check().await?;
            Ok(format!("{}: {}", health.service, health.status))
        }
    }
}

async fn run_repl(client: DlpClient) -> Result<()> {
    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();
    let mut stdout = io::stdout();

    stdout
        .write_all(b"DLP REPL. Type `help` for commands.\n")
        .await?;

    loop {
        stdout.write_all(b"dlp> ").await?;
        stdout.flush().await?;

        let Some(line) = lines.next_line().await? else {
            break;
        };

        match parse_interactive_command(&line) {
            Ok(InteractiveCommand::Health) => {
                let output = execute_command(Command::Health, &client).await?;
                stdout.write_all(output.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
            }
            Ok(InteractiveCommand::Help) => {
                stdout
                    .write_all(b"Commands: health, help, exit, quit\n")
                    .await?;
            }
            Ok(InteractiveCommand::Exit) => break,
            Err(error) => {
                stdout.write_all(error.to_string().as_bytes()).await?;
                stdout.write_all(b"\n").await?;
            }
        }
    }

    Ok(())
}

fn parse_interactive_command(input: &str) -> Result<InteractiveCommand> {
    match input.trim() {
        "" => bail!("enter a command"),
        "health" => Ok(InteractiveCommand::Health),
        "help" => Ok(InteractiveCommand::Help),
        "exit" | "quit" => Ok(InteractiveCommand::Exit),
        other => bail!("unknown command: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{InteractiveCommand, parse_interactive_command};

    #[test]
    fn parses_known_interactive_commands() {
        assert!(matches!(
            parse_interactive_command("health").expect("health"),
            InteractiveCommand::Health
        ));
        assert!(matches!(
            parse_interactive_command("help").expect("help"),
            InteractiveCommand::Help
        ));
        assert!(matches!(
            parse_interactive_command("quit").expect("quit"),
            InteractiveCommand::Exit
        ));
    }

    #[test]
    fn rejects_unknown_interactive_commands() {
        let error = parse_interactive_command("workers").expect_err("unknown command");
        assert!(error.to_string().contains("unknown command"));
    }
}
