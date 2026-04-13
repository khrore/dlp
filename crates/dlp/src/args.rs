#![expect(
    clippy::redundant_pub_crate,
    reason = "These CLI model types are shared between sibling modules through a private parent module."
)]

use clap::{Args as ClapArgs, Parser, Subcommand};
use dlp_api::shared::{DeviceClass, Framework, WorkloadMode};

#[derive(Debug, Parser)]
#[command(name = "dlp", about = "DLP client with shared CLI and REPL")]
pub(super) struct Args {
    #[arg(long, global = true)]
    pub api_host: Option<String>,

    #[arg(long, global = true)]
    pub api_port: Option<u16>,

    #[arg(long, global = true)]
    pub api_scheme: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum Command {
    #[command(subcommand)]
    Deployments(DeploymentsCommand),
    Health,
    #[command(subcommand)]
    Replicas(ReplicasCommand),
    #[command(subcommand)]
    Workers(WorkersCommand),
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum WorkersCommand {
    List,
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum DeploymentsCommand {
    Get(GetDeploymentArgs),
    Submit(SubmitDeploymentArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum ReplicasCommand {
    List(ListReplicasArgs),
}

#[derive(Debug, Clone, ClapArgs)]
pub(super) struct SubmitDeploymentArgs {
    #[arg(long)]
    pub accelerator_runtime: String,

    #[arg(long)]
    pub architecture_family: String,

    #[arg(long)]
    pub artifact_ref: String,

    #[arg(long)]
    pub concurrency: u32,

    #[arg(long)]
    pub device: DeviceClass,

    #[arg(long)]
    pub framework: Framework,

    #[arg(long)]
    pub memory_bytes: u64,

    #[arg(long)]
    pub mode: WorkloadMode,

    #[arg(long)]
    pub name: String,

    #[arg(long)]
    pub replicas: u32,
}

#[derive(Debug, Clone, ClapArgs)]
pub(super) struct GetDeploymentArgs {
    pub deployment_id: String,
}

#[derive(Debug, Clone, ClapArgs)]
pub(super) struct ListReplicasArgs {
    #[arg(long)]
    pub deployment_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) enum InteractiveCommand {
    Exit,
    Health,
    Help,
}
