# Repository Guidelines

## Project Structure & Module Organization

This repository is a Rust workspace rooted at `Cargo.toml`. Main crates live under `crates/`:

- `crates/app-config`: Figment-based config loading
- `crates/control-plane`: Axum server, application layer, and API entrypoint
- `crates/dlp-domain`: core domain model and invariants
- `crates/dlp-api`: shared HTTP and worker API DTOs
- `crates/dlp-client`: shared HTTP transport client for CLI, UI, and workers
- `crates/dlp`: shared CLI and REPL client
- `crates/pytorch-worker`: stub PyTorch worker binary used to exercise the control plane
- `crates/ui`: Leptos browser UI compiled to WASM

Docs belong in `docs/`. Generated UI assets currently land in `crates/ui/dist/`; avoid manual edits there unless the task is specifically about built output.

## Build, Test, and Development Commands

Prefer flake entrypoints so agents use the pinned toolchain and dependencies:

- `nix run .#fmt`: verify formatting with the flake-managed nightly rustfmt
- `nix run .#clippy`: run lint checks across the workspace
- `nix run .#test`: run unit and integration-style crate tests
- `nix run .#build`: build native workspace crates and compile the UI for `wasm32-unknown-unknown`
- `nix run .#check`: run the full `nix flake check` validation set
- `nix run .#control-plane`: start the API server
- `nix run .#dlp -- health`: run the CLI health check
- `nix run .#ui-dev`: start the Trunk dev server for `crates/ui`

Configuration loads from `config.toml` (searched upward) or `DLP_CONFIG_PATH`, with `DLP_CONTROL_PLANE_SERVER_*`, `DLP_DLP_API_*`, and `DLP_UI_API_*` overrides.

## Repository Rules

Repository-specific agent rules may live under `.rules/`. When a task touches persistence code, consult [.rules/seaorm-sql-boundaries.md](/Users/khrore/projects/dlp/.rules/seaorm-sql-boundaries.md) and follow its SeaORM versus raw SQL boundary rules.

## Agent Execution with Nix

When an agent needs to run validation, builds, or repository services, use `nix run .#<app>` from the repo root. Valid app attrs are `fmt`, `clippy`, `test`, `build`, `check`, `dlp`, `control-plane`, and `ui-dev`. Do not use bare host `cargo`, `rustfmt`, or `clippy` for repository workflows unless the task explicitly requires debugging outside the flake toolchain.

## Coding Style & Naming Conventions

Follow Rust 2024 conventions and run `nix run .#fmt` before submitting changes. Use 4-space indentation, `snake_case` for functions/modules, `PascalCase` for types, and kebab-case crate names. Prefer small, explicit APIs and avoid `unwrap`, `expect`, `panic!`, and `unsafe`; workspace lints deny or warn on them.

## Testing Guidelines

Tests are colocated with source using `#[cfg(test)]` modules. Name tests after observable behavior, for example `health_endpoint_returns_expected_payload`. Run `nix run .#test` before opening a PR, and add or update tests for behavior changes in CLI parsing, config loading, server handlers, or UI logic.

## Commit & Pull Request Guidelines

Recent history favors short, imperative commit subjects such as `Add typed app config with Figment` and occasional scoped docs commits like `docs: extend architecture for cli repl and gui`. Keep commit messages concise, present tense, and focused on one change.

PRs should include a brief summary, linked issue if applicable, validation commands run, and screenshots or notes for UI-visible changes. Call out config or migration impact explicitly.
