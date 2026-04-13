# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.
Do not rely on chat-only summaries, `/tmp` logs, or memory.

## Implementation Evidence

- Changed file: `Cargo.lock`
- Changed file: `Cargo.toml`
- Changed file: `crates/aspen-client-api/src/messages/mod.rs`
- Changed file: `crates/aspen-client-api/src/messages/request_metadata.rs`
- Changed file: `crates/aspen-client-api/src/messages/request_metadata_apps.rs`
- Changed file: `crates/aspen-client-api/tests/golden/request_discriminants.txt`
- Changed file: `crates/aspen-client-api/tests/golden/response_discriminants.txt`
- Changed file: `crates/aspen-client-api/tests/wire_format_golden.rs`
- Changed file: `crates/aspen-rpc-core/Cargo.toml`
- Changed file: `crates/aspen-rpc-core/src/context.rs`
- Changed file: `crates/aspen-rpc-core/src/handler.rs`
- Changed file: `crates/aspen-rpc-core/src/lib.rs`
- Changed file: `crates/aspen-rpc-core/src/service.rs`
- Changed file: `crates/aspen-rpc-handlers/Cargo.toml`
- Changed file: `crates/aspen-rpc-handlers/src/client.rs`
- Changed file: `crates/aspen-rpc-handlers/src/lib.rs`
- Changed file: `crates/aspen-rpc-handlers/src/registry.rs`
- Changed file: `src/lib.rs`
- Changed file: `crates/aspen-blob-handler/src/executor.rs`
- Changed file: `crates/aspen-blob-handler/src/lib.rs`
- Changed file: `crates/aspen-docs-handler/Cargo.toml`
- Changed file: `crates/aspen-docs-handler/src/lib.rs`
- Changed file: `crates/aspen-forge-handler/src/lib.rs`
- Changed file: `crates/aspen-forge-web/tests/patchbay_forge_web_test.rs`
- Changed file: `crates/aspen-ci-handler/src/lib.rs`
- Changed file: `crates/aspen-job-handler/Cargo.toml`
- Changed file: `crates/aspen-job-handler/src/lib.rs`
- Changed file: `crates/aspen-secrets-handler/src/lib.rs`
- Changed file: `crates/aspen-cluster-handler/src/lib.rs`
- Changed file: `crates/aspen-net/src/lib.rs`
- Changed file: `crates/aspen-nix-handler/src/lib.rs`
- Changed file: `crates/aspen-raft-network/src/types.rs`
- Changed file: `crates/aspen-raft-types/src/lib.rs`
- Changed file: `crates/aspen-raft/src/node/tests.rs`
- Changed file: `crates/aspen-raft/src/storage/in_memory/state_machine.rs`
- Changed file: `crates/aspen-raft/src/storage/in_memory/mod.rs`
- Changed file: `crates/aspen-raft/src/storage/mod.rs`
- Changed file: `crates/aspen-raft/src/types.rs`
- Changed file: `crates/aspen-transport/src/rpc.rs`
- Changed file: `crates/aspen-cluster/Cargo.toml`
- Changed file: `crates/aspen-cluster/src/lib.rs`
- Changed file: `crates/aspen-cluster/src/bootstrap/node/storage_init.rs`
- Changed file: `crates/aspen-cluster/src/bootstrap/node/sharding_init.rs`
- Changed file: `crates/aspen-cluster/src/bootstrap/node/mod.rs`
- Changed file: `crates/aspen-testing/src/router/mod.rs`
- Changed file: `crates/aspen-testing-madsim/src/madsim_tester/mod.rs`
- Changed file: `crates/aspen-testing-patchbay/src/node.rs`
- Changed file: `src/bin/aspen_node/main.rs`
- Changed file: `src/bin/aspen_node/setup/client.rs`
- Changed file: `src/bin/aspen_node/setup/router.rs`
- Changed file: `src/node/mod.rs`
- Changed file: `src/node/tests.rs`
- Changed file: `tests/consumer_group_integration_test.rs`
- Changed file: `tests/forge_real_cluster_integration_test.rs`
- Changed file: `tests/fuse_raft_cluster_test.rs`
- Changed file: `tests/gossip_e2e_integration.rs`
- Changed file: `tests/madsim_advanced_scenarios_test.rs`
- Changed file: `tests/madsim_append_entries_test.rs`
- Changed file: `tests/madsim_failure_injection_test.rs`
- Changed file: `tests/madsim_heartbeat_test.rs`
- Changed file: `tests/madsim_multi_node_test.rs`
- Changed file: `tests/madsim_replication_test.rs`
- Changed file: `tests/multi_node_cluster_test.rs`
- Changed file: `tests/pubsub_integration_test.rs`
- Changed file: `tests/relay_status_test.rs`
- Changed file: `tests/support/real_cluster.rs`
- Changed file: `tests/verify_integration_test.rs`
- Changed file: `tests/write_batching_integration.rs`
- Changed file: `scripts/check-cross-crate-boundaries.sh`
- Changed file: `openspec/changes/decouple-cross-crate-boundaries/tasks.md`
- Changed file: `openspec/changes/decouple-cross-crate-boundaries/verification.md`
- Changed file: `openspec/changes/decouple-cross-crate-boundaries/evidence/baseline-hotspots.txt`
- Changed file: `openspec/changes/decouple-cross-crate-boundaries/evidence/post-change-hotspots.txt`
- Changed file: `openspec/changes/decouple-cross-crate-boundaries/evidence/implementation-diff.txt`
- Changed file: `openspec/changes/decouple-cross-crate-boundaries/evidence/package-checks.txt`
- Changed file: `openspec/changes/decouple-cross-crate-boundaries/evidence/feature-slices.txt`
- Changed file: `openspec/changes/decouple-cross-crate-boundaries/evidence/acceptance-script.txt`
- Changed file: `openspec/changes/decouple-cross-crate-boundaries/evidence/bash-n-check-cross-crate-boundaries.txt`

## Task Coverage

- [x] Save a crate-graph and hotspot baseline for `aspen-client-api`, `aspen-rpc-core`, `aspen-rpc-handlers`, `aspen-cluster`, `src/bin/aspen_node/setup/client.rs`, and `src/node/mod.rs`, including direct evidence for the current god-context, mega-enum, feature-knot, and `unsafe` transmute seams.
  - Evidence: `openspec/changes/decouple-cross-crate-boundaries/evidence/baseline-hotspots.txt`, `openspec/changes/decouple-cross-crate-boundaries/evidence/implementation-diff.txt`

- [x] Add acceptance checks that future slices must keep green: client wire-format golden tests, representative feature-slice cargo checks, handler-registration tests, and bootstrap compile/start smoke tests.
  - Evidence: `scripts/check-cross-crate-boundaries.sh`, `crates/aspen-rpc-handlers/src/registry.rs`, `crates/aspen-client-api/tests/wire_format_golden.rs`, `src/node/tests.rs`, `openspec/changes/decouple-cross-crate-boundaries/evidence/acceptance-script.txt`, `openspec/changes/decouple-cross-crate-boundaries/evidence/feature-slices.txt`
  - Note: `scripts/check-cross-crate-boundaries.sh` now uses `--exact` test selection plus output assertions (`running 1 test`, `test ... ok`) so renamed/removed tests cannot false-pass with zero matches.

- [x] Move the shared Raft / transport type configuration behind one leaf crate boundary and remove the `transmute_raft_for_transport` `unsafe` bridge from `src/node/mod.rs`.
  - Evidence: `crates/aspen-raft-types/src/lib.rs`, `crates/aspen-raft/src/storage/in_memory/state_machine.rs`, `crates/aspen-raft/src/types.rs`, `crates/aspen-transport/src/rpc.rs`, `src/node/mod.rs`, `src/bin/aspen_node/setup/router.rs`, `openspec/changes/decouple-cross-crate-boundaries/evidence/post-change-hotspots.txt`, `openspec/changes/decouple-cross-crate-boundaries/evidence/implementation-diff.txt`

- [x] Split leaf features from convenience bundles in `crates/aspen-cluster/Cargo.toml`, `crates/aspen-rpc-core/Cargo.toml`, `crates/aspen-rpc-handlers/Cargo.toml`, and root feature wiring so direct prerequisites stay explicit and bundle names document cross-app composition.
  - Evidence: `Cargo.toml`, `crates/aspen-cluster/Cargo.toml`, `crates/aspen-rpc-core/Cargo.toml`, `crates/aspen-rpc-handlers/Cargo.toml`, `openspec/changes/decouple-cross-crate-boundaries/evidence/post-change-hotspots.txt`, `openspec/changes/decouple-cross-crate-boundaries/evidence/implementation-diff.txt`

- [x] Refactor `ClientProtocolContext` in `crates/aspen-rpc-core/src/context.rs` into declared capability traits or bounded subcontexts, and update test builders to construct only the capabilities under test.
  - Evidence: `crates/aspen-rpc-core/src/context.rs`, `crates/aspen-blob-handler/src/lib.rs`, `crates/aspen-docs-handler/src/lib.rs`, `crates/aspen-forge-handler/src/lib.rs`, `crates/aspen-ci-handler/src/lib.rs`, `crates/aspen-job-handler/src/lib.rs`, `crates/aspen-secrets-handler/src/lib.rs`, `openspec/changes/decouple-cross-crate-boundaries/evidence/post-change-hotspots.txt`, `openspec/changes/decouple-cross-crate-boundaries/evidence/implementation-diff.txt`

- [x] Move handler linking / registration ownership outward so `aspen-rpc-core` remains trait-focused and `aspen-rpc-handlers` or the binary layer assembles concrete domain handlers without reintroducing a global dependency bag.
  - Evidence: `crates/aspen-rpc-core/src/handler.rs`, `crates/aspen-rpc-core/src/lib.rs`, `crates/aspen-rpc-handlers/src/client.rs`, `crates/aspen-rpc-handlers/src/lib.rs`, `crates/aspen-rpc-handlers/src/registry.rs`, `src/bin/aspen_node/setup/client.rs`, `src/node/mod.rs`, `openspec/changes/decouple-cross-crate-boundaries/evidence/post-change-hotspots.txt`, `openspec/changes/decouple-cross-crate-boundaries/evidence/implementation-diff.txt`
  - Note: runtime composition now passes an explicit `NativeHandlerPlan` into `ClientProtocolHandler::new(...)`; registry assembly no longer probes `ClientProtocolContext` field presence to decide which native handlers exist.

- [x] Reduce the central ownership burden in `crates/aspen-client-api/src/messages/mod.rs` by moving request metadata and domain ownership toward domain-scoped modules while preserving golden-wire compatibility.
  - Evidence: `crates/aspen-client-api/src/messages/mod.rs`, `crates/aspen-client-api/src/messages/request_metadata.rs`, `crates/aspen-client-api/src/messages/request_metadata_apps.rs`, `crates/aspen-client-api/tests/golden/request_discriminants.txt`, `crates/aspen-client-api/tests/golden/response_discriminants.txt`, `crates/aspen-client-api/tests/wire_format_golden.rs`, `openspec/changes/decouple-cross-crate-boundaries/evidence/acceptance-script.txt`, `openspec/changes/decouple-cross-crate-boundaries/evidence/implementation-diff.txt`

- [x] Split `src/bin/aspen_node/setup/client.rs`, `crates/aspen-cluster/src/bootstrap/`, and `src/node/mod.rs` into explicit startup phases and per-domain installers so minimal node compositions can build without the current large feature conjunctions.
  - Evidence: `src/bin/aspen_node/setup/client.rs`, `crates/aspen-cluster/src/lib.rs`, `crates/aspen-cluster/src/bootstrap/node/mod.rs`, `src/node/mod.rs`, `Cargo.toml`, `openspec/changes/decouple-cross-crate-boundaries/evidence/feature-slices.txt`, `openspec/changes/decouple-cross-crate-boundaries/evidence/post-change-hotspots.txt`, `openspec/changes/decouple-cross-crate-boundaries/evidence/implementation-diff.txt`

- [x] Save post-change evidence showing reduced central edit hotspots, removed `unsafe` type bridging, explicit feature bundles, and bounded bootstrap phases.
  - Evidence: `openspec/changes/decouple-cross-crate-boundaries/evidence/post-change-hotspots.txt`, `openspec/changes/decouple-cross-crate-boundaries/evidence/implementation-diff.txt`, `openspec/changes/decouple-cross-crate-boundaries/evidence/acceptance-script.txt`

- [x] Run `scripts/openspec-preflight.sh decouple-cross-crate-boundaries` and add `verification.md` plus durable evidence before any task is checked complete.
  - Evidence: `openspec/changes/decouple-cross-crate-boundaries/verification.md`, `openspec/changes/decouple-cross-crate-boundaries/evidence/bash-n-check-cross-crate-boundaries.txt`, `openspec/changes/decouple-cross-crate-boundaries/evidence/acceptance-script.txt`, `openspec/changes/decouple-cross-crate-boundaries/evidence/openspec-preflight.txt`, `openspec/changes/decouple-cross-crate-boundaries/evidence/bash-n-openspec-preflight.txt`

## Review Scope Snapshot

### `git diff HEAD -- .`

- Status: captured
- Artifact: `openspec/changes/decouple-cross-crate-boundaries/evidence/implementation-diff.txt`

## Verification Commands

### `bash -n scripts/check-cross-crate-boundaries.sh`

- Status: pass
- Artifact: `openspec/changes/decouple-cross-crate-boundaries/evidence/bash-n-check-cross-crate-boundaries.txt`

### `cargo check -p aspen-rpc-handlers -p aspen-rpc-core -p aspen-blob-handler -p aspen-docs-handler -p aspen-forge-handler -p aspen-ci-handler -p aspen-job-handler -p aspen-secrets-handler -p aspen-transport -p aspen-raft-types -p aspen-raft -p aspen-nix-handler -p aspen-cluster --all-targets`

- Status: pass
- Artifact: `openspec/changes/decouple-cross-crate-boundaries/evidence/package-checks.txt`

### `cargo check -p aspen --all-targets --no-default-features && cargo check -p aspen --all-targets --no-default-features --features node-runtime && cargo test -p aspen --features node-runtime node::tests::test_nodebuilder_start_creates_node -- --exact --nocapture && cargo check -p aspen-client-api --test wire_format_golden && cargo check -p aspen-rpc-core --all-targets --features forge,hooks,git-bridge && cargo check -p aspen-rpc-handlers --all-targets --features blob,docs,forge,jobs,ci,secrets,net,snix`

- Status: pass
- Artifact: `openspec/changes/decouple-cross-crate-boundaries/evidence/feature-slices.txt`

### `./scripts/check-cross-crate-boundaries.sh`

- Status: pass
- Artifact: `openspec/changes/decouple-cross-crate-boundaries/evidence/acceptance-script.txt`

### `bash -n scripts/openspec-preflight.sh`

- Status: pass
- Artifact: `openspec/changes/decouple-cross-crate-boundaries/evidence/bash-n-openspec-preflight.txt`

### `scripts/openspec-preflight.sh decouple-cross-crate-boundaries`

- Status: pass
- Artifact: `openspec/changes/decouple-cross-crate-boundaries/evidence/openspec-preflight.txt`

## Notes

- Run `scripts/openspec-preflight.sh decouple-cross-crate-boundaries` after staging the new source and evidence files.
- `evidence/implementation-diff.txt` is a full `git diff HEAD -- .` snapshot for reviewable source evidence, not a narrowed file list.
- The acceptance script is the durable rail for wire golden checks, exact handler-registration tests, forge git-bridge feature wiring, and bootstrap compile/start smoke coverage.
