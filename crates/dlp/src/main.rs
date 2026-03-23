use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use client_sdk::DlpClient;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:3000";

#[derive(Debug, Parser)]
#[command(name = "dlp", about = "DLP client with shared CLI and REPL")]
struct Args {
    #[arg(
        long,
        global = true,
        env = "DLP_SERVER_URL",
        default_value = DEFAULT_SERVER_URL
    )]
    server_url: String,

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
    let client = DlpClient::new(args.server_url);

    match args.command {
        Some(command) => {
            let output = execute_command(command, &client).await?;
            println!("{output}");
        }
        None => run_repl(client).await?,
    }

    Ok(())
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
    use super::{parse_interactive_command, InteractiveCommand};

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

