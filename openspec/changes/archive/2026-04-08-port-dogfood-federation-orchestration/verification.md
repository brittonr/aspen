# Verification Evidence

## Implementation Evidence

- Changed file: `crates/aspen-dogfood/Cargo.toml`
- Changed file: `Cargo.lock`
- Changed file: `crates/aspen-dogfood/src/error.rs`
- Changed file: `crates/aspen-dogfood/src/forge.rs`
- Changed file: `crates/aspen-dogfood/src/ci.rs`
- Changed file: `crates/aspen-dogfood/src/main.rs`
- Changed file: `crates/aspen-dogfood/src/federation.rs`
- Changed file: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/tasks.md`
- Changed file: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/audit.md`
- Changed file: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/verification.md`
- Changed file: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/cargo-test-aspen-dogfood.txt`
- Changed file: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/federation-start.txt`
- Changed file: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/federation-push.txt`
- Changed file: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/federation-build.txt`
- Changed file: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/implementation-diff.txt`
- Changed file: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/bash-n-openspec-preflight.txt`
- Changed file: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/openspec-preflight.txt`

## Task Coverage

- [x] Compare `crates/aspen-dogfood` federation subcommands against `scripts/deprecated/dogfood-federation.sh` and list the still-unported orchestration steps.
  - Evidence: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/audit.md`
- [x] Identify where the Rust flow should create or mirror repos, trigger sync, and hand off the build to bob.
  - Evidence: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/audit.md`, `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/implementation-diff.txt`
- [x] Port the missing `federate` / `sync` / mirror-creation steps into the Rust binary.
  - Evidence: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/implementation-diff.txt`, `crates/aspen-dogfood/src/federation.rs`, `crates/aspen-dogfood/src/forge.rs`, `crates/aspen-dogfood/src/error.rs`
- [x] Ensure the build path in federation mode runs against the correctly mirrored repo on bob.
  - Evidence: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/implementation-diff.txt`, `crates/aspen-dogfood/src/main.rs`, `crates/aspen-dogfood/src/ci.rs`, `crates/aspen-dogfood/src/forge.rs`, `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/federation-build.txt`
- [x] Add regression tests for the pure orchestration helpers introduced by the port.
  - Evidence: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/implementation-diff.txt`, `crates/aspen-dogfood/src/federation.rs`, `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/cargo-test-aspen-dogfood.txt`
- [x] Run a federation-oriented verification flow that exercises the new orchestration path.
  - Evidence: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/federation-start.txt`, `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/federation-push.txt`, `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/federation-build.txt`
- [x] Capture command output or VM/integration evidence showing the mirrored-repo path works end to end.
  - Evidence: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/cargo-test-aspen-dogfood.txt`, `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/federation-start.txt`, `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/federation-push.txt`, `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/federation-build.txt`

## Review Scope Snapshot

### `git diff HEAD -- Cargo.lock crates/aspen-dogfood/Cargo.toml crates/aspen-dogfood/src/error.rs crates/aspen-dogfood/src/forge.rs crates/aspen-dogfood/src/ci.rs crates/aspen-dogfood/src/main.rs crates/aspen-dogfood/src/federation.rs openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/tasks.md`

- Status: captured
- Artifact: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/implementation-diff.txt`

## Verification Commands

### `cargo test -p aspen-dogfood`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/cargo-test-aspen-dogfood.txt`

### `nix run .#dogfood-federation -- --cluster-dir /tmp/aspen-dogfood-fed-inspect start`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/federation-start.txt`

### `nix run .#dogfood-federation -- --cluster-dir /tmp/aspen-dogfood-fed-inspect push`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/federation-push.txt`

### `nix run .#dogfood-federation -- --cluster-dir /tmp/aspen-dogfood-fed-inspect build`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/federation-build.txt`

### `bash -n scripts/openspec-preflight.sh`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/bash-n-openspec-preflight.txt`

### `scripts/openspec-preflight.sh openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-08-port-dogfood-federation-orchestration/evidence/openspec-preflight.txt`

## Notes

- The saved federation transcripts now cover start, push, and build. The build transcript shows Bob creating `aspen-mirror`, syncing from Alice, registering CI watch on the mirrored repo, and finding an auto-triggered Bob-side pipeline (`828060d4-363f-4c95-8103-fccdc0e40d9c`).
- `crates/aspen-dogfood/src/federation.rs` is new source code; it must be tracked by git before `nix run` / `nix build` so the flake source filter includes it.
- The review-scope diff artifact includes the checked source files plus `tasks.md`, so a reviewer can inspect the exact implementation delta without relying on chat context.
