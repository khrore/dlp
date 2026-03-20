# Deep Learning Platform (DLP)

Status: bootstrap stage. The repository now contains a minimal Rust workspace with a control plane, CLI, worker, and PostgreSQL-backed metadata loop for proving the first client-server slice.

## Current First Slice

The first runnable slice proves:

- job submission through a CLI client
- persistence of job and worker metadata in `PostgreSQL`
- a control plane in `Rust + Axum`
- a polling worker that claims a queued job, simulates execution, and reports a terminal result

This is intentionally not a full ML runtime yet. It validates the control-plane and execution-plane contract first.

## Quick Start

Enter the development shell:

```bash
nix develop
```

All commands below assume you are running inside that shell.

Point the control plane at a PostgreSQL database:

```bash
export DLP_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/dlp
```

Run the control plane:

```bash
cargo run -p dlp-control-plane
```

In another shell, start a worker:

```bash
cargo run -p dlp-worker
```

Submit a job:

```bash
cargo run -p dlp-cli -- submit --capability cpu --payload '{"prompt":"hello"}'
```

Check job status:

```bash
cargo run -p dlp-cli -- status <job-id>
```

Watch until completion:

```bash
cargo run -p dlp-cli -- watch <job-id>
```

On startup, the control plane creates the minimal `jobs` and `workers` tables it needs in the configured PostgreSQL database.

Deep Learning Platform (`dlp`) is a framework-agnostic, client-server platform for training, evaluation, inference, artifact management, and experiment operations across multiple machine learning runtimes.

The core platform does not depend on a single ML framework. Instead, it provides a Rust-based control plane and a set of pluggable execution workers for frameworks such as PyTorch, JAX, MLX, and MAX/Mojo.

## Goals

- Keep the core platform independent from any one ML framework
- Support both CPU and GPU execution through capability-based scheduling
- Support desktop and mobile clients through a shared UI stack
- Store models, checkpoints, datasets, and exports using S3-compatible object storage
- Separate orchestration concerns from training and inference runtimes
- Make it possible to add new ML backends without rewriting the platform core

## Technology Choices

- LLM and accelerated inference backend: `Mojo / MAX`
- Backend APIs and control plane: `Rust + Axum`
- UI: `Rust + Leptos`
- Native packaging for desktop and mobile: `Tauri`
- Object storage: `RustFS` using the `S3` protocol
- Metadata storage: `PostgreSQL`

## Architecture Summary

The platform is split into two major layers:

1. Control plane
2. Execution plane

### Control Plane

The control plane is implemented in `Rust` using `Axum`.

It is responsible for:

- authentication and authorization
- projects and workspaces
- experiment tracking
- job submission and orchestration
- worker registration and capability discovery
- artifact indexing and metadata management
- scheduling and lifecycle management
- API exposure for desktop, mobile, and web clients
- auditability, logging, and observability

The control plane must remain framework-agnostic. It should understand jobs, artifacts, datasets, workers, metrics, and lifecycle state, but not framework-specific tensor internals or training graph semantics.

### Execution Plane

The execution plane consists of independent workers. Each worker owns one runtime and reports back to the control plane.

Examples:

- `pytorch-worker`
- `jax-worker`
- `mlx-worker`
- `max-worker`

Workers are responsible for:

- loading the requested runtime
- preparing datasets and mounts
- running training, evaluation, inference, or export jobs
- emitting logs, metrics, and status events
- writing artifacts and checkpoints to object storage
- reporting final results back to the control plane

This design keeps the platform core clean and avoids coupling the system to one framework's assumptions.

## Core Design Decisions

### 1. Framework-Agnostic Core

The platform core models domain concepts such as:

- `Project`
- `Dataset`
- `ModelArtifact`
- `CheckpointArtifact`
- `TrainingJob`
- `InferenceJob`
- `EvaluationJob`
- `Worker`
- `WorkerCapability`

These concepts must not depend on PyTorch, JAX, MLX, or MAX-specific internals.

### 2. Adapter-Based Runtime Integration

Framework integration happens through worker adapters, not inside the control plane.

This allows the platform to support:

- PyTorch for broad training and production workflows
- JAX for research and accelerator-heavy workflows
- MLX for Apple silicon local workflows
- MAX/Mojo for optimized inference and future specialized compute paths

### 3. Capability-Based Scheduling

Jobs are scheduled by declared requirements and worker capabilities, not by hardcoded assumptions.

Examples of worker capabilities:

- `cpu`
- `cuda`
- `rocm`
- `apple-gpu`
- `multi-gpu`
- `distributed-training`
- `inference-only`

Examples of job requirements:

- framework
- accelerator type
- memory constraints
- distributed support
- network access policy
- storage access needs

### 4. Native Checkpoints, Explicit Portability

Training checkpoints should be treated as framework-native by default.

The platform should store:

- native checkpoint artifacts
- portable exports when available, such as `safetensors`
- sidecar metadata such as tokenizer, config, optimizer state, and lineage

The platform must not assume that a checkpoint can be resumed across frameworks unless an explicit conversion pipeline exists.

### 5. Separate Training and Inference Workloads

Training, evaluation, inference, and export should be modeled as distinct job classes.

This avoids forcing one execution model onto every workload and keeps scheduling, storage, and validation rules clear.

## Storage Model

### Object Storage

Use `RustFS` via the `S3` protocol for large binary assets:

- model weights
- checkpoints
- datasets
- logs
- exports
- evaluation outputs

### Metadata Storage

Use a relational database for structured metadata:

- projects
- runs
- job definitions
- worker inventory
- artifact manifests
- lineage
- metrics summaries
- access control records

This split keeps binary storage scalable while preserving queryable operational state.

## Client Applications

The primary UI should be built with `Leptos`.

Recommended client model:

- shared UI and app logic in Leptos
- native desktop and mobile packaging through `Tauri`
- optional browser-delivered web interface for operations and administration

This gives a mostly shared frontend while keeping deployment flexible across major platforms.

## Recommended Service Boundaries

An initial service layout can be:

- `control-plane`: Axum API server and orchestration logic
- `scheduler`: job placement and dispatch
- `artifact-service`: artifact metadata and storage integration
- `worker-gateway`: worker registration, heartbeats, and command delivery
- `ui`: Leptos frontend packaged with Tauri
- `workers/*`: framework-specific runtime workers

These may start as modules in a single deployable backend and split into separate services later if scale requires it.

## Non-Goals for the Core

The core should not:

- embed framework-specific training code
- promise cross-framework checkpoint resumability by default
- bind system behavior to one accelerator vendor
- require a single inference or training runtime

## Suggested V1 Scope

Version 1 should focus on:

- project and experiment management
- job submission and status tracking
- worker registration and capability discovery
- artifact and checkpoint storage
- basic metrics and logs
- one training backend, likely `PyTorch`
- one inference backend, potentially `MAX`
- desktop and web operator UI

This is the smallest credible product that preserves the long-term architecture.

## Future Expansion

Future versions can add:

- JAX execution workers
- MLX execution workers
- distributed training orchestration
- model conversion pipelines
- dataset versioning
- evaluation suites and benchmark packs
- policy-based scheduling
- multi-tenant resource governance

## Guiding Principle

`dlp` should be a platform for machine learning operations, not a wrapper around a single framework.

The control plane owns coordination. Workers own runtime execution. Storage owns durability. Clients own usability.
