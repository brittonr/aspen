## Context

The dogfood pipeline exercises Aspen's self-hosting loop: start cluster → push source to Forge → CI auto-triggers → nix build → deploy → verify. Currently the clippy CI job fails immediately with an empty error log. Two separate failures stack:

1. **Flake evaluation fails on Forge checkout.** When CI checks out source from Forge into a temp directory and runs `nix build .#checks.x86_64-linux.clippy`, the flake can't resolve properly. The Forge checkout lacks a `.git` directory, may be missing `flake.lock`, and the `ciSrc` filtering in `flake.nix` (which stubs/strips certain external repos) operates on different assumptions than a bare directory checkout.

2. **Build failure logs are lost.** `fetch_failure_logs` in `aspen-dogfood/src/ci.rs` fetches job logs via `CiGetJobLogs` but the CI executor's stderr capture for nix build failures doesn't reliably stream to the KV log store. The dogfood binary sees `job 'clippy' failed:` with empty content.

## Goals / Non-Goals

**Goals:**

- Clippy CI job passes when dogfood pushes source to Forge and CI runs `nix build .#checks.x86_64-linux.clippy`
- Failed CI jobs surface actual build stderr (nix build errors, clippy warnings) through `CiGetJobLogs`
- Dogfood pipeline prints the first ~50 lines of failure output so operators can diagnose without manual log retrieval

**Non-Goals:**

- Fixing the native snix build path (this change targets the `nix-cli-fallback` subprocess path used in dogfood)
- Making all 44 NixOS VM tests pass (separate effort)
- Supporting non-flake projects in CI

## Decisions

### D1: Fix Forge checkout to include flake.lock

The Forge `git push` creates a bare git repo on the server. When CI checks out a working copy, it needs to produce a directory that `nix build` can evaluate as a flake. This means the checkout must include `flake.lock` and the directory must either be a git repo (so nix can detect it as a git flake) or be referenced as `path:.` (so nix treats it as a path flake).

**Decision**: Ensure the CI executor's working directory for Forge checkouts is initialized as a git repo (`git init && git add . && git commit`) before running `nix build`. This matches what nix expects for `flake_url = "."`. The `flake.lock` file must be committed in the Forge push (verify `git-remote-aspen` doesn't filter it out).

**Alternative considered**: Use `path:.` instead of `.` as the flake URL. Rejected because path flakes don't get the same input resolution as git flakes and would require changes to the CI config schema.

### D2: Stream nix build stderr to KV log store

The `NixBuildWorker::execute_build` method captures stderr via a `mpsc::channel` and a `log_bridge` task that writes chunks to KV. When the build process fails quickly (e.g., nix evaluation error), the stderr lines may not flush before the worker returns `JobResult::failure`.

**Decision**: After the nix build subprocess exits with a non-zero code, drain the stderr reader fully before dropping the log sender. Add a small flush window (100ms) after process exit to capture trailing output. Store the last N lines of stderr in the `JobResult::failure` message itself as a fallback.

**Alternative considered**: Write logs synchronously instead of via channel. Rejected because it would block the build on KV writes.

### D3: Improve dogfood failure reporting

Currently `fetch_failure_logs` requests 50 log chunks and joins them. If no chunks exist (because of D2), the message is empty.

**Decision**: Fall back to the job's result message (from `CiGetStatus`) when `CiGetJobLogs` returns empty chunks. Print the failure detail to stderr with a clear header showing which job failed and why.

## Risks / Trade-offs

- **[Risk]** `git init` in CI working directory adds ~200ms overhead per job → Acceptable for correctness. Only needed for Forge checkouts, not local dev.
- **[Risk]** flake.lock might reference inputs not available in the Forge checkout → Mitigation: The `ciSrc` filtering in `flake.nix` already handles this by stubbing external repos. Verify the lock file's input hashes resolve from the binary cache.
- **[Risk]** Draining stderr after process exit could hang if the pipe buffer is full → Mitigation: Use `tokio::time::timeout` on the drain, cap at 1 second.
