use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use dlp_shared::{ClaimJobResponse, JobResultRequest, JobStatus, WorkerRegistration};
use reqwest::Url;
use serde_json::json;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value = "http://127.0.0.1:3000")]
    server: String,
    #[arg(long)]
    worker_id: Option<String>,
    #[arg(long, default_value = "local-worker")]
    name: String,
    #[arg(long, value_delimiter = ',', default_value = "cpu")]
    capabilities: Vec<String>,
    #[arg(long, default_value_t = 1000)]
    poll_ms: u64,
    #[arg(long, default_value_t = 1500)]
    work_ms: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let args = Args::parse();
    let server = Url::parse(&args.server).context("invalid --server URL")?;
    let worker_id = args.worker_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let client = reqwest::Client::new();

    let registration = WorkerRegistration {
        worker_id: worker_id.clone(),
        name: args.name,
        capabilities: args.capabilities,
    };

    client
        .post(server.join("/workers/register")?)
        .json(&registration)
        .send()
        .await?
        .error_for_status()?;

    info!("worker {worker_id} registered");

    loop {
        let response = client
            .post(server.join(&format!("/workers/{worker_id}/claim"))?)
            .send()
            .await?
            .error_for_status()?
            .json::<ClaimJobResponse>()
            .await?;

        if let Some(job) = response.job {
            info!("claimed job {}", job.job_id);
            tokio::time::sleep(Duration::from_millis(args.work_ms)).await;

            let result = JobResultRequest {
                worker_id: worker_id.clone(),
                success: true,
                result: Some(json!({
                    "message": "ok",
                    "job_kind": job.job_kind,
                })),
                error: None,
            };

            let completed = client
                .post(server.join(&format!("/jobs/{}/result", job.job_id))?)
                .json(&result)
                .send()
                .await?;

            if !completed.status().is_success() {
                warn!(
                    "failed to complete job: {}",
                    completed.text().await.unwrap_or_default()
                );
            } else {
                let job = completed.json::<dlp_shared::JobRecord>().await?;
                info!("job {} finished with status {:?}", job.job_id, job.status);
                if job.status != JobStatus::Completed {
                    warn!(
                        "job completed with unexpected terminal status {:?}",
                        job.status
                    );
                }
            }
        } else {
            tokio::time::sleep(Duration::from_millis(args.poll_ms)).await;
        }
    }
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dlp_worker=info".into()),
        )
        .init();
}
