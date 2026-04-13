# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.
Do not rely on chat-only summaries, `/tmp` logs, or memory.

## Implementation Evidence

- Changed file: `.agent/napkin.md`
- Changed file: `AGENTS.md`
- Changed file: `Cargo.toml`
- Changed file: `crates/aspen-client-api/src/messages/request_metadata_apps.rs`
- Changed file: `crates/aspen-client-api/src/messages/request_metadata_apps/automerge.rs`
- Changed file: `crates/aspen-client-api/src/messages/request_metadata_apps/calendar.rs`
- Changed file: `crates/aspen-client-api/src/messages/request_metadata_apps/ci.rs`
- Changed file: `crates/aspen-client-api/src/messages/request_metadata_apps/contacts.rs`
- Changed file: `crates/aspen-client-api/src/messages/request_metadata_apps/deploy.rs`
- Changed file: `crates/aspen-client-api/src/messages/request_metadata_apps/forge.rs`
- Changed file: `crates/aspen-client-api/src/messages/request_metadata_apps/hooks.rs`
- Changed file: `crates/aspen-client-api/src/messages/request_metadata_apps/jobs.rs`
- Changed file: `crates/aspen-client-api/src/messages/request_metadata_apps/secrets.rs`
- Changed file: `crates/aspen-client-api/src/messages/request_metadata_apps/snix.rs`
- Changed file: `crates/aspen-client-api/src/messages/request_metadata_apps/sql.rs`
- Changed file: `crates/aspen-cluster/Cargo.toml`
- Changed file: `crates/aspen-cluster/src/bootstrap/mod.rs`
- Changed file: `crates/aspen-cluster/src/bootstrap/node/discovery_init.rs`
- Changed file: `crates/aspen-cluster/src/bootstrap/node/hooks_init.rs`
- Changed file: `crates/aspen-cluster/src/bootstrap/node/mod.rs`
- Changed file: `crates/aspen-cluster/src/bootstrap/node/sync_init.rs`
- Changed file: `crates/aspen-cluster/src/bootstrap/resources.rs`
- Changed file: `crates/aspen-cluster/src/lib.rs`
- Changed file: `scripts/check-cross-crate-boundaries.sh`
- Changed file: `src/node/mod.rs`
- Changed file: `tests/support/mod.rs`
- Changed file: `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/acceptance-script.txt`
- Changed file: `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/domain-metadata-and-unsafe-check.txt`
- Changed file: `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/feature-slices.txt`
- Changed file: `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/implementation-diff.txt`
- Changed file: `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/openspec-preflight.txt`
- Changed file: `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/package-checks.txt`
- Changed file: `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/verification.md`
- Changed file: `openspec/specs/architecture-modularity/spec.md`

## Task Coverage

- [x] Save a crate-graph and hotspot baseline for `aspen-client-api`, `aspen-rpc-core`, `aspen-rpc-handlers`, `aspen-cluster`, `src/bin/aspen_node/setup/client.rs`, and `src/node/mod.rs`, including direct evidence for the current god-context, mega-enum, feature-knot, and `unsafe` transmute seams.
  - Evidence: `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/baseline-hotspots.txt`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/implementation-diff.txt`

- [x] Add acceptance checks that future slices must keep green: client wire-format golden tests, representative feature-slice cargo checks, handler-registration tests, and bootstrap compile/start smoke tests.
  - Evidence: `scripts/check-cross-crate-boundaries.sh`, `crates/aspen-rpc-handlers/src/registry.rs`, `crates/aspen-client-api/tests/wire_format_golden.rs`, `src/node/tests.rs`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/acceptance-script.txt`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/feature-slices.txt`
  - Note: `scripts/check-cross-crate-boundaries.sh` now uses `--exact` test selection plus output assertions (`running 1 test`, `test ... ok`) so renamed/removed tests cannot false-pass with zero matches.

- [x] Move the shared Raft / transport type configuration behind one leaf crate boundary and remove the `transmute_raft_for_transport` `unsafe` bridge from `src/node/mod.rs`.
  - Evidence: `crates/aspen-raft-types/src/lib.rs`, `crates/aspen-raft/src/storage/in_memory/state_machine.rs`, `crates/aspen-raft/src/types.rs`, `crates/aspen-transport/src/rpc.rs`, `src/node/mod.rs`, `src/bin/aspen_node/setup/router.rs`, `crates/aspen-cluster/src/bootstrap/node/sharding_init.rs`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/domain-metadata-and-unsafe-check.txt`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/post-change-hotspots.txt`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/implementation-diff.txt`
  - Note: `src/node/mod.rs` now has no `unsafe` / `transmute` hits, and sharded transport registration clones the shared `AppTypeConfig` Raft handle directly.

- [x] Split leaf features from convenience bundles in `crates/aspen-cluster/Cargo.toml`, `crates/aspen-rpc-core/Cargo.toml`, `crates/aspen-rpc-handlers/Cargo.toml`, and root feature wiring so direct prerequisites stay explicit and bundle names document cross-app composition.
  - Evidence: `Cargo.toml`, `crates/aspen-cluster/Cargo.toml`, `crates/aspen-rpc-core/Cargo.toml`, `crates/aspen-rpc-handlers/Cargo.toml`, `scripts/check-cross-crate-boundaries.sh`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/feature-slices.txt`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/post-change-hotspots.txt`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/implementation-diff.txt`
  - Note: root `node-runtime` now compiles the bootstrap core slice, while `node-runtime-apps` is the explicit named bundle for jobs/docs/hooks/federation composition.

- [x] Refactor `ClientProtocolContext` in `crates/aspen-rpc-core/src/context.rs` into declared capability traits or bounded subcontexts, and update test builders to construct only the capabilities under test.
  - Evidence: `crates/aspen-rpc-core/src/context.rs`, `crates/aspen-blob-handler/src/lib.rs`, `crates/aspen-docs-handler/src/lib.rs`, `crates/aspen-forge-handler/src/lib.rs`, `crates/aspen-ci-handler/src/lib.rs`, `crates/aspen-job-handler/src/lib.rs`, `crates/aspen-secrets-handler/src/lib.rs`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/post-change-hotspots.txt`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/implementation-diff.txt`

- [x] Move handler linking / registration ownership outward so `aspen-rpc-core` remains trait-focused and `aspen-rpc-handlers` or the binary layer assembles concrete domain handlers without reintroducing a global dependency bag.
  - Evidence: `crates/aspen-rpc-core/src/handler.rs`, `crates/aspen-rpc-core/src/lib.rs`, `crates/aspen-rpc-handlers/src/client.rs`, `crates/aspen-rpc-handlers/src/lib.rs`, `crates/aspen-rpc-handlers/src/registry.rs`, `src/bin/aspen_node/setup/client.rs`, `src/node/mod.rs`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/post-change-hotspots.txt`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/implementation-diff.txt`
  - Note: runtime composition now passes an explicit `NativeHandlerPlan` into `ClientProtocolHandler::new(...)`; registry assembly no longer probes `ClientProtocolContext` field presence to decide which native handlers exist.

- [x] Reduce the central ownership burden in `crates/aspen-client-api/src/messages/mod.rs` by moving request metadata and domain ownership toward domain-scoped modules while preserving golden-wire compatibility.
  - Evidence: `crates/aspen-client-api/src/messages/mod.rs`, `crates/aspen-client-api/src/messages/request_metadata.rs`, `crates/aspen-client-api/src/messages/request_metadata_apps.rs`, `crates/aspen-client-api/src/messages/request_metadata_apps/forge.rs`, `crates/aspen-client-api/src/messages/request_metadata_apps/jobs.rs`, `crates/aspen-client-api/src/messages/request_metadata_apps/secrets.rs`, `crates/aspen-client-api/src/messages/request_metadata_apps/snix.rs`, `crates/aspen-client-api/tests/golden/request_discriminants.txt`, `crates/aspen-client-api/tests/golden/response_discriminants.txt`, `crates/aspen-client-api/tests/wire_format_golden.rs`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/domain-metadata-and-unsafe-check.txt`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/acceptance-script.txt`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/implementation-diff.txt`
  - Note: `request_metadata_apps.rs` now dispatches to per-domain files, so adding a Forge RPC updates Forge-owned metadata instead of a mixed cross-domain table.

- [x] Split `src/bin/aspen_node/setup/client.rs`, `crates/aspen-cluster/src/bootstrap/`, and `src/node/mod.rs` into explicit startup phases and per-domain installers so minimal node compositions can build without the current large feature conjunctions.
  - Evidence: `src/bin/aspen_node/setup/client.rs`, `crates/aspen-cluster/src/lib.rs`, `crates/aspen-cluster/src/bootstrap/mod.rs`, `crates/aspen-cluster/src/bootstrap/node/mod.rs`, `crates/aspen-cluster/src/bootstrap/node/discovery_init.rs`, `crates/aspen-cluster/src/bootstrap/node/hooks_init.rs`, `crates/aspen-cluster/src/bootstrap/node/sync_init.rs`, `src/node/mod.rs`, `Cargo.toml`, `tests/support/mod.rs`, `scripts/check-cross-crate-boundaries.sh`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/feature-slices.txt`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/post-change-hotspots.txt`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/implementation-diff.txt`
  - Note: `cargo check -p aspen --all-targets --no-default-features --features node-runtime` now exercises the minimal bootstrap slice, while `node-runtime-apps` names the broader app bundle explicitly.

- [x] Save post-change evidence showing reduced central edit hotspots, removed `unsafe` type bridging, explicit feature bundles, and bounded bootstrap phases.
  - Evidence: `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/post-change-hotspots.txt`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/implementation-diff.txt`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/acceptance-script.txt`

- [x] Run `scripts/openspec-preflight.sh decouple-cross-crate-boundaries` and add `verification.md` plus durable evidence before any task is checked complete.
  - Evidence: `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/verification.md`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/bash-n-check-cross-crate-boundaries.txt`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/acceptance-script.txt`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/openspec-preflight.txt`, `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/bash-n-openspec-preflight.txt`

## Review Scope Snapshot

### `git diff HEAD -- .`

- Status: captured
- Artifact: `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/implementation-diff.txt`

## Verification Commands

### `bash -n scripts/check-cross-crate-boundaries.sh`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/bash-n-check-cross-crate-boundaries.txt`

### `cargo check -p aspen-rpc-handlers -p aspen-rpc-core -p aspen-blob-handler -p aspen-docs-handler -p aspen-forge-handler -p aspen-ci-handler -p aspen-job-handler -p aspen-secrets-handler -p aspen-transport -p aspen-raft-types -p aspen-raft -p aspen-nix-handler -p aspen-cluster --all-targets`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/package-checks.txt`

### `cargo check -p aspen --all-targets --no-default-features && cargo check -p aspen --all-targets --no-default-features --features node-runtime && cargo check -p aspen --all-targets --no-default-features --features node-runtime-apps && cargo test -p aspen --features node-runtime node::tests::test_nodebuilder_start_creates_node -- --exact --nocapture && cargo check -p aspen-client-api --test wire_format_golden && cargo check -p aspen-rpc-core --all-targets --features forge,hooks,git-bridge && cargo check -p aspen-rpc-handlers --all-targets --features blob,docs,forge,jobs,ci,secrets,net,snix`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/feature-slices.txt`

### `./scripts/check-cross-crate-boundaries.sh`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/acceptance-script.txt`

### `bash -n scripts/openspec-preflight.sh`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/bash-n-openspec-preflight.txt`

### `scripts/openspec-preflight.sh decouple-cross-crate-boundaries`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-13-decouple-cross-crate-boundaries/evidence/openspec-preflight.txt`

## Notes

- Run `scripts/openspec-preflight.sh decouple-cross-crate-boundaries` after staging the new source and evidence files.
- `evidence/implementation-diff.txt` is a full `git diff HEAD -- .` snapshot for reviewable source evidence, not a narrowed file list.
- The acceptance script is the durable rail for wire golden checks, exact handler-registration tests, forge git-bridge feature wiring, and bootstrap compile/start smoke coverage.
