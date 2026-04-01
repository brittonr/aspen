## Context

The dogfood pipeline validates that Aspen can build and host itself: start a cluster, push source to Forge, trigger CI, build via Nix, deploy the resulting binary back to the cluster. Today this is implemented as three large bash scripts that shell out to `aspen-cli` and parse text output.

The `aspen-client` crate already exposes every RPC the dogfood scripts use (`GetHealth`, `InitCluster`, `WriteKey`, `ScanKeys`, CI operations, Forge git init/list, federation, deploy, blob operations). The CLI is just a thin wrapper — the dogfood scripts are doing typed operations through an untyped text layer.

Current script structure:

- `dogfood-local.sh` (1,278 lines): single-cluster dogfood with subcommands start/stop/push/build/deploy/verify/full-loop
- `dogfood-local-vmci.sh` (935 lines): same but CI jobs run in Cloud Hypervisor microVMs
- `dogfood-federation.sh` (895 lines): two-cluster federation dogfood (alice pushes, bob syncs and builds)

~60% of the logic is shared: cluster startup, health polling, forge repo creation, CI monitoring, artifact verification.

## Goals / Non-Goals

**Goals:**

- Replace all three dogfood scripts with a single `aspen-dogfood` binary crate
- Use `aspen-client` for cluster communication (no `aspen-cli` subprocess)
- Share common orchestration logic across single-node, VM-CI, and federation modes
- Maintain the same `nix run .#dogfood-local` UX (subcommand interface unchanged)
- Typed error handling with snafu instead of grep + exit codes

**Non-Goals:**

- Replacing the small scripts (`cluster-start.sh`, `cluster-stop.sh`, `cluster-form.sh`, `cluster-test.sh`) — these are short and appropriate as bash
- Building a general-purpose orchestration library — this is dogfood-specific
- Changing the dogfood test logic or adding new test scenarios (pure refactor)
- Replacing the NixOS VM integration tests — those stay as Nix test nodes

## Decisions

### 1. Single binary with mode flags, not three binaries

The three scripts share a common structure (start → push → build → verify). The federation variant adds a second cluster; the VM-CI variant changes the executor type. A single binary with `--federation` and `--vm-ci` flags is cleaner than three binaries or three feature flags.

**Alternative considered:** Three separate binaries. Rejected because the shared logic is >60% and would need a shared library crate anyway, at which point the binaries are trivial wrappers.

### 2. Use `aspen-client` directly, not `aspen-cli` as a subprocess

The client crate has typed request/response enums for every operation the dogfood scripts use. Going through CLI means serializing to text and parsing it back — a lossy round-trip that causes silent failures when output format changes.

**Alternative considered:** Keep shelling out to `aspen-cli` but from Rust (using `std::process::Command`). Rejected — this preserves all the fragility of the bash approach. The whole point is to use the typed API.

### 3. Process management via `tokio::process` + PID tracking

The current scripts manage node processes with PID files, `kill -0` polling, and trap handlers. The Rust equivalent: spawn child processes via `tokio::process::Command`, store `Child` handles, use `tokio::signal` for SIGTERM/SIGINT cleanup, and `child.wait()` for lifecycle tracking.

**Alternative considered:** Use `nix` crate for lower-level process control. Overkill — `tokio::process` handles everything needed.

### 4. Structured logging with `tracing` instead of colored echo

The bash scripts use colored `echo` with emoji prefixes (✅ ❌ ⚠️). The Rust binary uses `tracing` with a human-friendly subscriber (`tracing-subscriber` with `fmt::Layer`). Keep the emoji/color output style — it's readable in terminal sessions.

### 5. Crate location: `crates/aspen-dogfood/`

Follows the existing convention (`crates/aspen-cli/`, `crates/aspen-tui/`). Single binary target: `aspen-dogfood`.

## Risks / Trade-offs

**[Risk] Compile time adds latency to dogfood iteration** → The binary is built as part of the Nix flake app, same as `aspen-node` and `aspen-cli`. Incremental rebuilds are ~2-3s. No worse than current setup where the scripts depend on pre-built binaries.

**[Risk] Harder to make quick one-off tweaks than editing a shell script** → True, but the scripts haven't been "quick tweaks" for a long time at 900-1,300 lines. The type safety payoff outweighs the edit-run cycle cost.

**[Risk] `aspen-client` API gaps** → If any dogfood operation turns out to be CLI-only without a client API equivalent, we add it to `aspen-client-api`. This is a feature, not a risk — it means the client API is actually complete.

**[Trade-off] Larger dependency closure for dogfood binary** → The binary pulls in `aspen-client`, `iroh`, `tokio`, `clap`, `snafu`, `tracing`. This is ~30s first build. Acceptable for an internal dev tool, and the dependencies are already in the workspace.
