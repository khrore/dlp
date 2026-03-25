# DLP Implementation Architecture

This document describes the concrete implementation architecture for the first phase of `dlp`.

The immediate target is to launch and manage multiple concurrent AI model instances. In this phase, the control plane owns desired state, scheduling, and lifecycle coordination, while execution workers own runtime startup and shutdown on actual compute nodes.

## V1 Runtime Topology

Version 1 should stay close to the current workspace layout and avoid premature microservice splits.

Runtime-relevant workspace units:

- `control-plane` from `crates/control-plane`: public API, desired state, scheduler module, and worker-gateway module
- `dlp` from `crates/dlp`: CLI and REPL client
- `client-sdk` from `crates/client-sdk`: shared request and response contracts for all clients
- future shared worker-agent library or module: generic lifecycle, heartbeats, leases, and process supervision
- future `pytorch-worker`: a thin binary that embeds the shared worker agent plus the PyTorch runtime provider
- future `max-worker`: a thin binary that embeds the shared worker agent plus the MAX runtime provider

For V1, `scheduler`, `artifact-service`, and `worker-gateway` should remain internal modules inside the `control-plane` deployable. Split them into separate services only after scale or isolation needs are proven.

## Control Plane Responsibilities

For multi-instance model serving and batch execution, the control plane should own:

- model deployment definitions and desired replica counts
- placement decisions across available workers
- leases for model-instance slots on workers
- lifecycle state for deployments, replicas, and jobs
- rollout coordination such as create, scale, drain, restart, and delete

The control plane should not directly embed PyTorch or MAX runtimes, and it should not be responsible for spawning framework processes through framework-specific code paths. Its role is to tell a worker what to run, with what resources, and with what artifact references.

## Worker Model

For the first version, treat a worker as a node-local agent with two responsibilities:

- advertise node capabilities such as framework support, accelerator inventory, memory, and local concurrency limits
- host one or more runtime providers that can start and stop concrete model instances

Every worker agent should implement the same control-plane-facing contract, regardless of whether it hosts PyTorch or MAX.

Minimum worker operations:

- register supported providers and hardware capabilities
- heartbeat current health, inventory, and available concurrency slots
- accept `prepare`, `launch`, `probe`, `stop`, and `evict` commands
- stream logs, metrics, and lifecycle events
- publish terminal status plus produced artifact references

Minimum worker capability fields for V1:

- `framework`: `pytorch` or `max`
- `mode`: `training` or `inference`
- `device`: `cpu`, `cuda`, or later other accelerator classes
- available memory
- concurrency slot count
- optional artifact cache inventory

Recommended worker states:

- `starting`
- `ready`
- `draining`
- `unhealthy`
- `lost`

## Runtime Provider Contract

Define a worker-side `RuntimeProvider` abstraction with a narrow lifecycle contract:

- validate a workload spec against local capabilities
- prepare runtime inputs such as model artifacts, tokenizer files, and environment variables
- start a model instance or batch job
- expose readiness and health state
- stream logs and metrics
- stop and clean up the runtime

Initial providers:

- `PyTorchProvider`: first for training jobs and simple inference processes
- `MaxProvider`: first for optimized inference deployments

Artifact compatibility should stay explicit:

- training jobs usually produce framework-native artifacts first
- provider-specific prepared artifacts are optional cached derivatives
- conversion from PyTorch outputs into MAX-ready inference artifacts should be modeled as an explicit export pipeline, not an implicit assumption

This keeps framework-specific environment bootstrapping, Python bindings, model server command lines, and accelerator quirks outside the control plane.

## Scheduling Model

Jobs are scheduled by declared requirements and worker capabilities, not by hardcoded assumptions.

For multi-instance placement, extend scheduling inputs with:

- framework
- accelerator type
- memory constraints
- distributed support
- network access policy
- storage access needs
- estimated per-instance memory footprint
- exclusive or shared GPU policy
- warm-cache preference for reused model artifacts
- maximum replicas per worker
- spread and anti-affinity rules
- priority and preemption policy

## Deployment and Replica Model

To support multiple concurrently running model instances, add a deployment-oriented resource model alongside one-shot jobs.

Recommended core resources:

- `ModelDeployment`: desired long-lived serving definition
- `ModelReplica`: one concrete running instance of a deployment on a worker
- `Job`: bounded execution such as training, evaluation, export, or batch inference
- `Worker`: a registered execution agent on a node
- `WorkerLease`: a reservation of worker capacity for a replica or job
- `RuntimeProviderRef`: provider type requested by the workload, such as `pytorch` or `max`

Recommended lifecycle split:

- deployments reconcile toward desired replica count
- replicas move through `pending`, `pulling`, `starting`, `ready`, `draining`, `stopped`, or `failed`
- jobs move through `queued`, `assigned`, `running`, `succeeded`, `failed`, or `cancelled`

This allows `dlp` to manage both long-lived model serving and short-lived execution without forcing them into the same lifecycle abstraction.

## Model Instance Lifecycle

Launching multiple model instances is not just a `start process` action. The system needs continuous reconciliation.

Recommended loop:

1. the user or API creates a deployment or job
2. the scheduler selects candidate workers based on capabilities and free capacity
3. the control plane creates an assignment and lease
4. the target worker accepts the assignment and uses the requested runtime provider
5. the worker reports state transitions and health
6. the control plane reconciles failures by retrying, rescheduling, or scaling down

The minimum lifecycle for one model instance should be:

1. a user submits a deployment or batch inference request
2. the control plane validates the request and selects a worker
3. the control plane acquires or creates a `WorkerLease`
4. the worker runs `prepare` to fetch artifacts and reserve local resources
5. the provider launches one model instance
6. the worker reports `starting`, then `ready` or `failed`
7. the worker streams logs and metrics while the instance is active
8. the instance is either torn down after completion or kept warm according to policy
9. the worker publishes final status and any artifacts before releasing the lease

Failure ownership rules should also be explicit:

- if `prepare` succeeds and `launch` fails, the worker releases reserved local resources and reports the lease as failed
- if a worker dies while holding a `WorkerLease`, the control plane expires the lease after heartbeat timeout and reschedules according to policy
- if a replica turns unhealthy after `ready`, the control plane may restart it in place or replace it on another worker
- if contact is lost during `draining`, the control plane treats the replica as uncertain and reconciles toward the desired state rather than assuming clean shutdown

## Orchestration Boundary

`dlp` should have its own orchestration layer at the application level, because it needs domain-aware scheduling decisions around frameworks, accelerators, artifact locality, and workload class.

That does not mean `dlp` must build a full cluster manager from scratch.

Recommended approach:

- keep `dlp` scheduling, desired state, and worker lifecycle decisions inside the control plane
- introduce a pluggable infrastructure launcher under the worker or agent layer
- support a simple local or VM process launcher first
- add Kubernetes as an infrastructure backend, not as the architecture itself

This leads to two distinct layers:

- application orchestration: deployments, replicas, jobs, leases, capabilities, policies
- infrastructure orchestration: processes, containers, pods, volumes, node placement primitives

Kubernetes should be treated as optional infrastructure, not as the source of truth for `dlp` scheduling.

Boundary of responsibility:

- `dlp` control plane owns workload intent, desired replicas, placement policy, worker leases, and lifecycle reconciliation
- worker agents own local runtime supervision and provider-specific process management
- Kubernetes, when used, owns pod scheduling and container restart primitives underneath worker agents

In V1, local processes or VM-hosted workers managed directly by `dlp` are sufficient. Kubernetes becomes useful only after the worker protocol and reconciliation behavior are stable.

## Suggested V1 Scope

Version 1 should focus on:

- job submission and status tracking
- deployment submission and replica tracking
- worker registration and capability discovery
- artifact and checkpoint storage
- basic metrics and logs
- one worker agent implementation with local process supervision
- one training backend, `PyTorch`
- one inference backend, `MAX`
- a command-style CLI

For the immediate milestone of launching multiple concurrent model instances, project and experiment management, REPL polish, and GUI packaging can remain out of scope until the worker protocol and reconciliation loop are proven.

Suggested rollout order:

- Phase 1: `PyTorch` training worker
- Phase 2: `MAX` inference worker
- Phase 3: mixed-fleet scheduling across both providers

## V1 Roadmap

| Phase | New crate or module | Responsibility | Exit criterion |
| --- | --- | --- | --- |
| 1 | `crates/control-plane` scheduler and worker-gateway modules | placement, leases, heartbeats, command dispatch | control plane can assign work to one worker and track lifecycle state |
| 2 | `crates/client-sdk` job, deployment, replica, and worker models | shared API contracts for CLI and UI | clients can submit deployments and inspect worker and replica state |
| 3 | `crates/pytorch-worker` | worker agent plus `PyTorchProvider` for training and simple inference | one node can run and supervise multiple PyTorch instances |
| 4 | `crates/max-worker` | worker agent plus `MaxProvider` for optimized inference | one node can run and supervise multiple MAX replicas |
| 5 | `crates/control-plane` mixed-fleet scheduling policies | schedule across PyTorch and MAX workers with capability checks | control plane can place training and inference workloads across a small worker pool |
