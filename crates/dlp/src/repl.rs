use anyhow::{Result, bail};
use dlp_client::DlpClient;
use tokio::io::{self, AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};

use crate::{
    args::{Command, InteractiveCommand},
    commands::execute_command,
};

/// Runs the interactive REPL loop for the DLP client.
///
/// # Errors
///
/// Returns an error when stdin, stdout, or a client request fails.
pub async fn run_repl(client: DlpClient) -> Result<()> {
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

/// Parses a single REPL command line into an interactive command.
///
/// # Errors
///
/// Returns an error when the input is empty or does not match a supported
/// command.
pub fn parse_interactive_command(input: &str) -> Result<InteractiveCommand> {
    match input.trim() {
        "" => bail!("enter a command"),
        "health" => Ok(InteractiveCommand::Health),
        "help" => Ok(InteractiveCommand::Help),
        "exit" | "quit" => Ok(InteractiveCommand::Exit),
        other => bail!("unknown command: {other}"),
    }
}
