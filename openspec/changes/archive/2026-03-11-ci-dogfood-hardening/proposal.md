## Why

The CI dogfood pipeline — Aspen building itself using its own Forge + CI — has 11 identified flaws ranging from dead config paths to executor misrouting. The most critical: the NixOS VM dogfood test exercises shell jobs only, never validating the `type = 'nix` code path that production actually uses. This means the self-hosting litmus test proves a different code path than what ships.

## What Changes

- **Shell worker log streaming**: Add `_ci:logs:*` KV chunk writes to `LocalExecutorWorker` so `ci logs --follow` works for shell jobs (currently only NixBuildWorker streams logs)
- **Job type routing fix**: Remove `ci_vm` and `ci_nix_build` from `LocalExecutorWorker.job_types()` so shell executor doesn't steal jobs meant for VM/Nix workers
- **Dogfood VM test uses Nix executor**: Rewrite `ci-dogfood.nix` to use `type = 'nix` jobs that go through `NixBuildWorker`, matching the real `.aspen/ci.ncl`
- **`ignore_paths` evaluation**: Implement the `ignore_paths` trigger filter that exists in config but is never checked
- **Checkout cleanup on completion**: Clean up `/tmp/ci-checkout-*` on pipeline success/failure, not just cancellation
- **Watch-before-push race**: Buffer gossip announcements received before `ci watch` and replay on watch
- **SNIX_GIT_SOURCE drift guard**: Add compile-time or CI check that `checkout.rs` constant matches `flake.nix`

## Capabilities

### New Capabilities

- `ci-shell-log-streaming`: Real-time log streaming for shell executor jobs via KV log chunks
- `ci-ignore-paths-filter`: Evaluate `ignore_paths` from trigger config against changed files to skip docs-only pushes

### Modified Capabilities

- `ci`: Job type routing — shell executor only claims `shell_command` and `local_executor` types; checkout cleanup on terminal pipeline states
- `forge-ci-trigger`: Evaluate `ignore_paths` during trigger; buffer pre-watch gossip to prevent race

## Impact

- `crates/aspen-ci-executor-shell/src/local_executor/mod.rs` — job_types, log streaming
- `crates/aspen-ci/src/trigger/service.rs` — gossip buffering, ignore_paths evaluation
- `crates/aspen-ci/src/checkout.rs` — SNIX_GIT_SOURCE validation
- `crates/aspen-ci/src/orchestrator/pipeline/status.rs` — cleanup on completion
- `nix/tests/ci-dogfood.nix` — rewrite to use Nix executor
- `.aspen/ci.ncl` — no changes (already correct, test was wrong)
