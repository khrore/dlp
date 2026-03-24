# Repository Guidelines

## Project Structure & Module Organization

This repository is a Rust workspace rooted at `Cargo.toml`. Main crates live under `crates/`:

- `crates/control-plane`: Axum server and API entrypoint
- `crates/dlp`: shared CLI and REPL client
- `crates/client-sdk`: request/response models and transport client
- `crates/app-config`: Figment-based config loading
- `crates/ui`: Leptos browser UI compiled to WASM

Docs belong in `docs/`. Generated UI assets currently land in `crates/ui/dist/`; avoid manual edits there unless the task is specifically about built output.

## Build, Test, and Development Commands

Prefer flake entrypoints for runnable services so agents use the pinned toolchain and dependencies:

- `nix run .#control-plane`: start the API server
- `nix run .#dlp -- health`: run the CLI health check
- `nix run .#ui-dev`: start the Trunk dev server for `crates/ui`
- `cargo fmt --all --check`: verify formatting
- `cargo clippy --workspace --all-targets`: run lint checks across the workspace
- `cargo test --workspace`: run unit and integration-style crate tests
- `cargo build -p ui --target wasm32-unknown-unknown`: compile the UI for the browser

Configuration loads from `config.toml` (searched upward) or `DLP_CONFIG_PATH`, with `DLP_CONTROL_PLANE_SERVER_*`, `DLP_DLP_API_*`, and `DLP_UI_API_*` overrides.

## Agent Execution with Nix

When an agent needs to run a repository service, use `nix run .#<service>` from the repo root. Valid app attrs are `dlp`, `control-plane`, and `ui-dev`. Use Cargo directly for formatting, linting, and tests unless the task needs the full dev shell.

## Coding Style & Naming Conventions

Follow Rust 2024 conventions and run `cargo fmt` before submitting changes. Use 4-space indentation, `snake_case` for functions/modules, `PascalCase` for types, and kebab-case crate names. Prefer small, explicit APIs and avoid `unwrap`, `expect`, `panic!`, and `unsafe`; workspace lints deny or warn on them.

## Testing Guidelines

Tests are colocated with source using `#[cfg(test)]` modules. Name tests after observable behavior, for example `health_endpoint_returns_expected_payload`. Run `cargo test --workspace` before opening a PR, and add or update tests for behavior changes in CLI parsing, config loading, server handlers, or UI logic.

## Commit & Pull Request Guidelines

Recent history favors short, imperative commit subjects such as `Add typed app config with Figment` and occasional scoped docs commits like `docs: extend architecture for cli repl and gui`. Keep commit messages concise, present tense, and focused on one change.

PRs should include a brief summary, linked issue if applicable, validation commands run, and screenshots or notes for UI-visible changes. Call out config or migration impact explicitly.
