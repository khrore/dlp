# DLP Storage Architecture

This document defines the concrete V1 storage design for `dlp` control-plane metadata and artifact storage.

It complements, rather than replaces, the durable platform architecture in `docs/ARCHITECTURE.md` and the broader phase-one implementation plan in `docs/IMPLEMENTATION_ARCHITECTURE.md`.

## Overview

Version 1 uses a split storage model:

- PostgreSQL is the canonical metadata store
- S3-compatible object storage is the binary artifact store

PostgreSQL is the source of truth for scheduler-visible state, worker lifecycle state, leases, assignments, and reconciliation inputs. Object storage holds large binary assets only. The control plane should persist references to objects, metadata, and lifecycle state in PostgreSQL rather than embedding large payloads in relational rows.

Recorded V1 defaults:

- scope: V1 pragmatic
- metadata backend: PostgreSQL-first
- artifact backend: S3-compatible object storage
- migration strategy: repository swap behind current seams
- service boundary: persistence remains in-process inside the control plane
- API stability: no intended HTTP API changes as part of the storage transition

## Design Goals

The V1 storage design should optimize for:

- durability across control-plane restarts
- correctness for leases, heartbeats, assignments, and reconciliation
- minimal churn to the current application and repository structure
- a clean extension path for later metadata domains such as projects, experiments, and access control

The storage layer should support the current control-plane workflow without forcing premature service splits, event-sourcing infrastructure, or database-specific behavior in the public API.

## Storage Model

### Metadata in PostgreSQL

PostgreSQL stores structured operational metadata for the current control plane:

- deployments
- replicas
- leases
- workers
- worker capabilities
- worker assignments

These records are authoritative for:

- desired replica counts
- replica placement and lifecycle state
- active worker inventory and heartbeat freshness
- worker-local slot reservations through leases
- assignment delivery to workers on heartbeat

Metadata rows must store typed object references and structured fields, not raw binary artifact contents.

### Binary Artifacts in Object Storage

S3-compatible object storage stores large binary assets:

- model weights
- checkpoints
- datasets
- exports
- logs
- evaluation outputs

PostgreSQL should store object references, manifests, and lifecycle metadata for these assets. V1 should not store blobs in PostgreSQL except for ordinary structured metadata fields whose size is operationally small.

## V1 Metadata Schema Scope

The initial PostgreSQL schema should cover the hot path already present in the control plane.

### Core tables

`deployments`

- stores deployment identity, user-visible name, artifact reference, desired replica count, workload requirement fields, and timestamps
- is the source record for deployment reconciliation

`replicas`

- stores replica identity, owning deployment ID, current lifecycle state, assigned worker ID when present, lease ID when present, status message, and timestamps
- is the source record for deployment status rollups and worker ownership checks

`leases`

- stores lease identity, worker ID, replica ID, lease state, requirement snapshot, and timestamps
- is the source record for capacity reservation and lease release behavior

`workers`

- stores worker identity, display name, current worker state, last heartbeat timestamp, and timestamps
- is the source record for active inventory, lost-worker detection, and worker restart handling

`worker_capabilities`

- stores one row per worker capability entry, including framework, mode, device, accelerator runtime, architecture family, available memory, and concurrency slots
- is the source record for scheduler eligibility checks

`worker_assignments`

- stores durable queued assignments for delivery to a worker, including worker ID, replica ID, lease ID, payload, creation ordering, and delivery state
- is the source record for atomic enqueue and drain-on-heartbeat behavior

### Identifier and state rules

- all primary IDs should remain stable string identifiers compatible with current domain IDs such as `deployment-1`, `replica-2`, `lease-3`, and `worker-1`
- lifecycle state values must align with current domain state enums rather than introducing storage-only meanings
- timestamps should be stored in a form suitable for ordering, lease freshness checks, and lost-worker detection

### Hot-path indexes

The schema should include indexes that match the current reconcile and scheduling flow:

- replicas by `deployment_id`
- replicas by lifecycle `state`, especially pending replicas
- leases by `worker_id` where lease state is active
- leases by `worker_id` plus requirement fields for capacity checks
- workers by `state`
- workers by `last_heartbeat_at`
- worker assignments by `worker_id` and creation order

If partial indexes or equivalent PostgreSQL features are used, they should optimize active-lease and pending-replica lookups without changing control-plane behavior.

## Control-Plane Integration

The current control plane already uses repository traits in `crates/control-plane/src/repositories/mod.rs` and a concrete in-memory implementation in `crates/control-plane/src/repositories/memory.rs`. V1 storage should preserve that repository pattern and replace the concrete persistence backend behind it.

Required integration direction:

- preserve the repository boundaries for deployments, replicas, leases, and workers
- replace the direct `MemoryStore` dependency in `crates/control-plane/src/application/mod.rs` with a storage abstraction or repository-backed shared state
- keep the in-memory store available for tests and fast local-only scenarios
- make PostgreSQL the default durable runtime path for the control plane

The goal is a repository swap, not an application rewrite. The control plane should continue to own scheduling, reconciliation, and worker-gateway behavior in-process while persisting durable state through repository implementations backed by PostgreSQL.

## Transaction Boundaries

Correctness depends on explicit transaction boundaries. The following operations must be atomic.

### Deployment creation plus initial replicas

One transaction should:

- insert the deployment row
- create the initial pending replica rows required by desired capacity
- persist any status fields derived from the initial state

If any step fails, the deployment must not appear partially created.

### Replica assignment plus lease creation plus assignment enqueue

One transaction should:

- select an eligible worker using current durable state
- create the active lease row
- update the replica with assigned worker and lease identity
- enqueue the worker assignment row
- persist any deployment or worker summary fields that depend on the assignment

If any step fails, no partial assignment may remain visible.

### Replica terminal transition plus lease release

One transaction should:

- apply the replica state update
- validate lease ownership against the replica
- release the lease when the replica enters a terminal state such as `failed` or `stopped`
- persist any derived status updates required by the owning deployment

If the terminal transition cannot be committed completely, neither the replica nor the lease should advance.

### Worker registration conflict handling and restart expiration

One transaction should:

- detect whether a worker registration conflicts with an existing live worker identity
- expire impacted active leases and owned assignments when restart rules require it
- transition affected replicas back to pending or failed state according to control-plane policy
- write the new worker registration and capability rows

This prevents restart races from leaving orphaned leases or stale queued assignments.

Across all cases, partial writes within one logical control-plane action must roll back completely.

## Configuration and Operations

V1 should introduce dedicated configuration for metadata storage and keep it separate from object-storage configuration.

### Metadata database configuration

The control plane should load explicit PostgreSQL connection settings, such as:

- DSN or structured host, port, database, user, password, and TLS options
- connection pool sizing
- migration behavior required at startup

### Object-storage configuration

Object storage configuration should remain separate and include endpoint, bucket, credentials, and any path-prefix or region-like settings needed by the chosen S3-compatible backend.

### Startup and runtime behavior

- required migrations must be checked at startup
- the control plane must fail fast on unreachable PostgreSQL, invalid credentials, or unapplied required migrations
- the control plane must not silently fall back to in-memory persistence in production mode
- V1 does not introduce a separate storage service; persistence remains inside the control-plane process

## Testing and Validation

The storage design is complete only if it is validated independently from the in-memory implementation.

### Repository validation

- repository tests for all PostgreSQL-backed CRUD and query behavior
- coverage for deployments, replicas, leases, workers, capability rows, and worker assignment queues
- explicit tests for state filtering, ordering, and heartbeat freshness behavior

### Transactional integration tests

- deployment creation creates the expected pending replicas
- scheduling assigns only eligible workers
- slot exhaustion keeps extra replicas pending
- worker loss expires active leases and restores pending work correctly
- replica lifecycle updates propagate to deployment summaries correctly
- worker assignment queues drain atomically on heartbeat

### Startup and configuration failure tests

- unreachable PostgreSQL
- invalid database configuration
- missing required migrations
- object-storage misconfiguration when artifact operations are exercised

### End-to-end verification

Run the control plane with PostgreSQL and a worker implementation and verify:

- worker registration
- heartbeat-driven assignment delivery
- deployment submission
- replica transitions to `ready`
- failure and replacement behavior where applicable

## Future Expansion

V1 should leave clear room for later metadata domains without requiring a redesign of the core storage split.

Reserved next-stage metadata domains:

- projects and workspaces
- runs and experiments
- artifact manifests and lineage
- metrics summaries
- access control records

These domains should be added as new PostgreSQL tables and relationships that extend the same metadata authority model. They are intentionally out of the V1 implementation scope of this document.
