# Deep Learning Platform (DLP)

Deep Learning Platform (`dlp`) is a framework-agnostic, client-server platform for training, evaluation, inference, artifact management, and experiment operations across multiple machine learning runtimes.

The core platform does not depend on a single ML framework. Instead, it provides a Rust-based control plane and a set of pluggable execution workers for frameworks such as PyTorch, JAX, MLX, and MAX/Mojo.

This document describes the durable, higher-level architecture of the platform. Concrete implementation decisions for the first delivery phase live in `docs/IMPLEMENTATION_ARCHITECTURE.md`.

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
- Metadata storage: relational database

## Architecture Summary

The platform is split into three major layers:

1. Interface layer
2. Control plane
3. Execution plane

### Interface Layer

The interface layer provides multiple operator-facing entry points over the same platform services and domain model.

It should include:

- a command-style `dlp` CLI for scripting, automation, and CI/CD usage
- an interactive shell / `REPL` for exploratory operations, debugging, and guided workflows
- a GUI for desktop, mobile, and web-based operations

These interfaces should not duplicate orchestration logic. They should all talk to the same application API and use the same domain concepts such as jobs, workers, artifacts, datasets, and experiments.

Recommended internal split:

- `cli`: command parser, flags, output formatting, and non-interactive command execution
- `repl`: session management, command history, contextual prompts, and interactive helpers
- `gui`: visual workflows, dashboards, forms, and monitoring views
- shared client SDK or application service layer: authentication, request models, transport, error handling, and common business actions

This keeps user interaction concerns separate from backend orchestration while allowing all operator surfaces to evolve together.

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

The execution plane consists of independent workers. Workers own runtime execution and report operational state back to the control plane.

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

The platform should expose three first-class client modes:

- command-style CLI
- interactive shell / `REPL`
- GUI

The primary GUI should be built with `Leptos`.

Recommended client model:

- shared domain client and API bindings used by CLI, REPL, and GUI
- CLI commands for automation-friendly workflows such as `submit`, `status`, `logs`, `artifacts`, and `workers`
- REPL commands that reuse the CLI command model but add session state, shortcuts, discovery, and guided interaction
- shared UI and app logic in Leptos
- native desktop and mobile packaging through `Tauri`
- optional browser-delivered web interface for operations and administration

This gives one platform with multiple operator experiences instead of separate products with diverging behavior.

## Recommended Service Boundaries

An initial service layout can be:

- `control-plane`: Axum API server and orchestration logic
- `scheduler`: job placement and dispatch
- `artifact-service`: artifact metadata and storage integration
- `worker-gateway`: worker registration, heartbeats, and command delivery
- `client-sdk`: shared API bindings, auth flows, transport, and domain operations for all clients
- `cli`: command-style interface built on the shared client SDK
- `repl`: interactive shell built on the shared client SDK and CLI command primitives
- `ui`: Leptos frontend packaged with Tauri
- `workers/*`: framework-specific runtime workers

These may start as modules in a single deployable backend and split into separate services later if scale requires it.

## Interaction Model

Each user-facing surface serves a different operational mode:

- CLI: deterministic commands, scripts, CI jobs, and machine-readable output
- REPL: iterative investigation, admin workflows, live inspection, and operator assistance
- GUI: dashboards, forms, visual monitoring, and multi-step workflows

All three should map to the same backend capabilities. If a job can be submitted in the GUI, it should also be possible through the CLI and REPL unless there is a deliberate product restriction.

## Non-Goals for the Core

The core should not:

- embed framework-specific training code
- promise cross-framework checkpoint resumability by default
- bind system behavior to one accelerator vendor
- require a single inference or training runtime

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
