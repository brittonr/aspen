## Context

Aspen has 80 crates, 436k lines of Rust, a 4,300-line monolithic `flake.nix`, and informal design docs in `docs/`. The CI agent communicates with VMs over vsock using hand-written serde types. The deploy executor assumes uniform deployment strategy across all targets. NixOps4's rewrite surfaced patterns that address each of these gaps — this design covers how to adopt them without disrupting existing code.

Current state:

- **Flake**: Single `flake.nix` using `flake-utils`, no `flake-parts` modules
- **Docs**: `docs/` has design docs (federation, forge, plugins, etc.) but no structured ADR format
- **Schema**: `schemars` already in 9 crate Cargo.toml files; used for CI config → Nickel schema generation
- **Deploy**: `DeployExecutor` in `aspen-ci` operates on flat `DeployRequest` with uniform strategy
- **CI agent**: `protocol.rs` defines `HostMessage`/`AgentMessage` as plain serde types; no schema export

## Goals / Non-Goals

**Goals:**

- Establish ADR conventions and retroactively document foundational decisions
- Break `flake.nix` into composable modules without changing build outputs
- Generate JSON Schema from existing protocol types and add schema validation to CI agent IPC
- Let deploy targets opt into stateful lifecycle tracking vs. stateless push deploys
- Isolate Nix evaluation/build in a supervised child process within the CI orchestrator

**Non-Goals:**

- Adopting OpenTofu or Terraform providers (Aspen manages resources via Raft)
- Replacing Iroh with HTTP (Iroh-only networking is a core architectural invariant)
- Introducing a Nix module system for resource composition (Aspen uses Rust/Nickel)
- Rewriting the plugin system from scratch (extend existing Hyperlight interface with schemas)
- Migrating away from `crane` to `nix-cargo-integration` (crane works well for the workspace)

## Decisions

### D1: ADR format and conventions

Use the format from `adr.github.io` — Context, Decision, Consequences — with a numbered prefix (`001-`, `002-`, etc.). Store in `docs/adr/`. Each ADR is a standalone markdown file.

Retroactive ADRs to write (~10):

1. Iroh-only networking (no HTTP)
2. Vendored openraft v0.10
3. Redb unified log + state machine
4. Functional Core Imperative Shell
5. Verus two-file architecture
6. Tiger Style resource bounds
7. Nickel for CI configuration
8. Content-addressed blobs via iroh-blobs
9. ALPN-based protocol routing
10. Madsim deterministic simulation

**Alternative considered**: RFCs (heavier process with discuss/accepted/rejected lifecycle). Rejected — ADRs are lower friction and sufficient for recording decisions already made.

### D2: flake-parts modularization

Migrate from `flake-utils` to `flake-parts`. Split the monolithic flake into modules:

| Module file | Concern |
|---|---|
| `nix/flake-modules/rust.nix` | Crane builds, cargo artifacts, per-crate packages |
| `nix/flake-modules/checks.nix` | Clippy, formatting, advisory audit |
| `nix/flake-modules/tests.nix` | NixOS VM integration tests (44 tests) |
| `nix/flake-modules/dev.nix` | Dev shell, tooling |
| `nix/flake-modules/dogfood.nix` | Dogfood pipeline, serial VM |
| `nix/flake-modules/verus.nix` | Verus verification, inline check |
| `nix/flake-modules/vms.nix` | VM images, microvm configs |
| `nix/flake-modules/apps.nix` | Runnable apps (cluster, fuzz, coverage) |

The root `flake.nix` becomes a thin shell that imports these modules and declares inputs. Build outputs stay identical — this is a pure structural refactor.

**Alternative considered**: Keep `flake-utils` and just split into `let` bindings/imports. Rejected — `flake-parts` gives proper module system composition with `perSystem`, `flake`, and typed options. It's also what the ecosystem is converging on.

**Migration approach**: Incremental — extract one module at a time starting with `dev.nix` (lowest risk), verify `nix flake check` passes after each extraction.

### D3: Schema-driven IPC for CI agent protocol

Derive JSON Schema from the existing serde types in `protocol.rs` using `schemars`. The schema serves as the contract between host and guest — any protocol change that alters the schema is a visible diff in code review.

Steps:

1. Add `#[derive(JsonSchema)]` to `HostMessage`, `AgentMessage`, `ExecutionRequest`, `ExecutionResult`, `LogMessage` in `protocol.rs`
2. Add a `#[test]` that serializes the schema to `schemas/ci-agent-protocol.json` and asserts it matches the checked-in version (snapshot test)
3. Optionally generate Rust types from the schema on the guest side via `typify` as a build.rs step — validates round-trip compatibility

This extends the existing `schemars` usage (already in 9 crates) rather than introducing a new dependency.

**Alternative considered**: Protobuf or Cap'n Proto for IPC. Rejected — the vsock protocol is already JSON-based with length-prefixed frames. Adding a binary serialization format would be a larger change with marginal benefit for the current message sizes.

### D4: Per-resource statefulness in deploy executor

Add a `stateful: bool` field to `DeployRequest`. When `stateful: true`, the executor writes lifecycle state to Raft KV under `_deploy:state:{deploy_id}:{resource}` — tracking versions, rollback points, and health history. When `stateful: false`, it's a fire-and-forget push deploy with no persistent state beyond the job log.

The Nickel CI config gains a `deploy.stateful` option (default: `true` for backwards compatibility). The deploy executor branches on this flag in `execute()`.

**Alternative considered**: Make statefulness a property of the resource type rather than the deploy config. Rejected — the same resource (e.g., an aspen-node binary) might be deployed statelessly in dev and statefully in production. It's a deployment decision, not a resource property.

### D5: CI orchestrator process isolation for Nix builds

The CI orchestrator currently runs Nix builds as subprocess commands but manages them inline. Formalize this by:

1. Moving Nix evaluation/build into a supervised child process managed by a `NixBuildSupervisor`
2. The supervisor communicates with the orchestrator via a channel (not vsock — this is same-host)
3. If the Nix process hangs past `NIX_BUILD_TIMEOUT` (configurable, default 30 min), the supervisor kills it and reports failure without crashing the orchestrator
4. Pipeline state in Raft KV survives the Nix process death — the orchestrator can mark the job failed and continue processing other pipeline stages

This mirrors NixOps4's evaluator process separation but adapted for Aspen's architecture (channels instead of IPC, Raft KV for state persistence instead of local DB).

**Alternative considered**: Run Nix builds inside the VM worker (already isolated). For `ci-basic` (shell executor) builds, there's no VM — the build runs on the node itself. Process isolation protects against Nix hangs in this mode without requiring a VM.

## Risks / Trade-offs

- **[flake-parts migration]** Unfamiliar module system for contributors → Mitigate with ADR documenting the decision and inline comments in each module file
- **[Schema snapshots]** Schema snapshot tests break on any protocol change → Intentional: forces protocol changes to be visible. Update the snapshot when the change is deliberate.
- **[Deploy statefulness]** Two code paths (stateful/stateless) increase testing surface → Mitigate by keeping the stateless path as the simple case (no KV writes) and testing both paths in the existing NixOS VM integration tests
- **[Process isolation]** Channel-based supervisor adds complexity to the orchestrator → Mitigate by keeping the supervisor as a thin wrapper over `tokio::process::Command` with timeout logic, not a full process manager
- **[Retroactive ADRs]** Writing ADRs for decisions already made can feel like busywork → Keep them short (half a page each). The value is for future contributors who need to understand "why" without reading 436k lines of code.
