use std::time::Duration;

use anyhow::Context;
use clap::{Parser, Subcommand};
use dlp_shared::{JobKind, JobRecord, SubmitJobRequest};
use reqwest::Url;
use serde_json::Value;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value = "http://127.0.0.1:3000")]
    server: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Submit {
        #[arg(long, value_delimiter = ',')]
        capability: Vec<String>,
        #[arg(long, default_value = "{}")]
        payload: String,
    },
    Status {
        job_id: String,
    },
    Watch {
        job_id: String,
        #[arg(long, default_value_t = 1000)]
        poll_ms: u64,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let server = Url::parse(&args.server).context("invalid --server URL")?;
    let client = reqwest::Client::new();

    match args.command {
        Command::Submit {
            capability,
            payload,
        } => {
            let payload: Value = serde_json::from_str(&payload).context("invalid JSON payload")?;
            let request = SubmitJobRequest {
                job_kind: JobKind::DummyInference,
                required_capabilities: capability,
                payload,
            };

            let job = client
                .post(server.join("/jobs")?)
                .json(&request)
                .send()
                .await?
                .error_for_status()?
                .json::<JobRecord>()
                .await?;

            println!("{}", serde_json::to_string_pretty(&job)?);
        }
        Command::Status { job_id } => {
            let job = fetch_job(&client, &server, &job_id).await?;
            println!("{}", serde_json::to_string_pretty(&job)?);
        }
        Command::Watch { job_id, poll_ms } => loop {
            let job = fetch_job(&client, &server, &job_id).await?;
            println!("{}", serde_json::to_string_pretty(&job)?);
            if matches!(
                job.status,
                dlp_shared::JobStatus::Completed | dlp_shared::JobStatus::Failed
            ) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(poll_ms)).await;
        },
    }

    Ok(())
}

async fn fetch_job(
    client: &reqwest::Client,
    server: &Url,
    job_id: &str,
) -> anyhow::Result<JobRecord> {
    Ok(client
        .get(server.join(&format!("/jobs/{job_id}"))?)
        .send()
        .await?
        .error_for_status()?
        .json::<JobRecord>()
        .await?)
}
