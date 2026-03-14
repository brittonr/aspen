## Why

Aspen's `flake.nix` is a 4,300-line monolith, architectural decisions live as scattered informal docs, and the deploy/plugin interfaces lack schema-driven contracts. NixOps4's rewrite surfaced several patterns that address these gaps — ADRs for decision traceability, `flake-parts` for flake modularity, schema-first IPC for plugin stability, and a per-resource statefulness model for deploy flexibility. Adopting these patterns now reduces friction as the codebase grows past 80 crates.

## What Changes

- Introduce Architecture Decision Records (ADRs) in `docs/adr/` with numbered, structured format (Context/Decision/Consequences). Retroactively document ~10 foundational decisions (Iroh-only networking, redb storage, vendored openraft, FCIS, Verus two-file, Tiger Style).
- Split `flake.nix` into composable `flake-parts` modules — separate flake modules for Rust builds, NixOS VM tests, dev tooling, dogfood pipeline, and Verus verification.
- Extend schema-driven type generation from CI config (existing `aspen-generate-schema`) to cover the plugin RPC interface and CI agent↔orchestrator protocol, using JSON Schema as the contract.
- Add per-resource statefulness to the deploy executor — deploy targets declare `stateful: true/false` rather than the executor assuming a uniform strategy. Stateful targets get lifecycle tracking in Raft KV; stateless targets get simple push deploys.
- Formalize CI agent process separation — the CI orchestrator and Nix evaluation/build execution run as supervised child processes so a stuck `nix build` can be killed without losing pipeline state.

## Capabilities

### New Capabilities

- `adr-framework`: Architecture Decision Record framework with numbered format, retroactive documentation of foundational decisions, and conventions for new ADRs.
- `flake-modularization`: Split monolithic `flake.nix` into `flake-parts` modules organized by concern (build, test, dev, dogfood, verify).
- `schema-driven-plugin-ipc`: JSON Schema contracts for Hyperlight plugin RPC and CI agent↔orchestrator protocol, with Rust type codegen via `typify`/`schemars`.
- `deploy-resource-statefulness`: Per-resource statefulness model for the deploy executor, replacing the current uniform deployment assumption.
- `ci-process-isolation`: Process boundary between CI orchestrator and Nix evaluation/build execution, with supervised child processes and independent failure domains.

### Modified Capabilities

- `ci`: Deploy executor gains per-resource statefulness; CI agent gains process isolation for Nix builds.

## Impact

- `flake.nix` — major restructuring into multiple files under `nix/flake-modules/`
- `crates/aspen-ci/src/orchestrator/deploy_executor.rs` — new statefulness model per deploy target
- `crates/aspen-ci/src/agent/` — process supervision for Nix eval/build
- `crates/aspen-ci/src/config/` — schema generation extended to RPC messages
- Plugin interface (`crates/aspen-plugins/` or equivalent) — schema-first contract
- `docs/adr/` — new directory with ~10 retroactive ADRs
- No breaking API changes to existing RPC protocols — schema generation wraps existing types
