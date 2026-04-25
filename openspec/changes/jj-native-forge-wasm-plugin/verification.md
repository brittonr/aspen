# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.
Do not rely on chat-only summaries, `/tmp` logs, or memory.

## Implementation Evidence

- Changed file: `crates/aspen-forge-protocol/src/lib.rs`
- Changed file: `crates/aspen-client-api/src/messages/mod.rs`
- Changed file: `crates/aspen-client-api/src/messages/request_metadata.rs`
- Changed file: `crates/aspen-client-api/src/messages/request_metadata_apps/forge.rs`
- Changed file: `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json`
- Changed file: `crates/aspen-forge-handler/src/executor.rs`
- Changed file: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-jj-native-admission-tests.txt`
- Changed file: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-rustfmt-check.txt`
- Changed file: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-openspec-preflight.txt`
- Changed file: `openspec/changes/jj-native-forge-wasm-plugin/tasks.md`
- Changed file: `openspec/changes/jj-native-forge-wasm-plugin/verification.md`

## Task Coverage

- [x] 1.1 Extend Forge repo metadata, repository list responses, and client-facing capability discovery so a repo can declare `git`, `jj`, or both backends and return node-specific routing identifiers for active backends.
  - Evidence: `crates/aspen-forge-protocol/src/lib.rs`, `crates/aspen-forge-handler/src/executor.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-forge-plugin-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-rustfmt-check.txt`

- [x] 1.3 Default or backfill pre-existing repositories with no backend manifest to `git`-only behavior.
  - Evidence: `crates/aspen-forge-handler/src/executor.rs`, `crates/aspen-forge-protocol/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-forge-plugin-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-rustfmt-check.txt`

- [x] 1.6 Reject protocol-identifier collisions during plugin registration/activation and surface a deterministic error.
  - Evidence: `crates/aspen-plugin-api/src/manifest.rs`, `crates/aspen-plugin-api/src/resolve.rs`, `crates/aspen-cli/src/bin/aspen-cli/commands/plugin.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-forge-plugin-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-cli-plugin-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-rustfmt-check.txt`

- [x] 2.1 Define the native JJ object model and blob encoding used by Forge for commits, trees, files, conflicts, and related metadata.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-1-aspen-forge-jj-object-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-1-rustfmt-check.txt`

- [x] 2.2 Implement repo-scoped JJ object persistence and reachability lookup on top of BLAKE3-addressed blobs.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-2-aspen-forge-jj-store-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-2-rustfmt-check.txt`

- [x] 2.4 Implement Raft-backed JJ change-id indexes and JJ bookmark namespaces, including create/move/delete semantics.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-4-aspen-forge-jj-index-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-4-rustfmt-check.txt`

- [x] 2.7 Add optimistic final-publish conflict checks for stale bookmark heads and stale change-id heads.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-7-aspen-forge-jj-conflict-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-7-rustfmt-check.txt`

- [x] 2.6 Validate incoming JJ payloads and reject malformed or inconsistent object graphs before final publish.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-6-aspen-forge-jj-graph-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-6-rustfmt-check.txt`

- [x] 2.5 Implement staged JJ push publication so partial or failed streams do not make repo-visible state inconsistent.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-5-aspen-forge-jj-staged-publish-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-5-rustfmt-check.txt`

- [x] 1.7 Persist the manifest-declared JJ routing identifier in plugin registration metadata, surface it in capability discovery, and keep it stable across reload/upgrade when unchanged.
  - Evidence: `crates/aspen-cli/src/bin/aspen-cli/commands/plugin.rs`, `crates/aspen-forge-handler/src/executor.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-7-1-8-forge-handler-route-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-7-1-8-rustfmt-check.txt`

- [x] 1.8 Publish and withdraw node-local JJ activation state so discovery only advertises nodes with an active JJ plugin.
  - Evidence: `crates/aspen-forge-handler/src/executor.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-7-1-8-forge-handler-route-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-7-1-8-rustfmt-check.txt`

- [x] 2.8 Add staged-data quota, expiry, and cleanup behavior for successful publish, timeout, rejection, and abandoned-session paths.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-8-aspen-forge-jj-staged-cleanup-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-8-rustfmt-check.txt`

- [x] 2.9 Abort or reject in-flight JJ sessions when a repo is deleted or JJ support is disabled, and block final publish after that transition.
  - Evidence: `crates/aspen-forge/src/jj.rs`, `crates/aspen-forge/src/lib.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-9-aspen-forge-jj-repo-state-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-9-rustfmt-check.txt`

- [x] 1.4 Add JJ-native request/response families, transport identifiers, and explicit transport-version advertisement/compatibility checks for clone, fetch, push, bookmark sync, and change-id lookup.
  - Evidence: `crates/aspen-forge-protocol/src/lib.rs`, `crates/aspen-client-api/src/messages/mod.rs`, `crates/aspen-client-api/src/messages/request_metadata.rs`, `crates/aspen-client-api/src/messages/request_metadata_apps/forge.rs`, `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json`, `crates/aspen-forge-handler/src/executor.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-jj-native-admission-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-rustfmt-check.txt`

## Review Scope Snapshot

Review remediation after same-family review: tasks 1.1, 1.4, and 1.6 are intentionally unchecked until routing discovery, session-admission compatibility checks, and lifecycle registration/activation enforcement have durable implementation evidence.

## Verification Commands

### `cargo test -p aspen-plugin-api -p aspen-forge-protocol -p aspen-forge-handler`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-forge-plugin-tests.txt`

### `cargo test -p aspen-cli --features plugins-rpc <focused plugin tests>`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-cli-plugin-tests.txt`

### `rustfmt --check crates/aspen-cli/src/bin/aspen-cli/commands/plugin.rs crates/aspen-forge-handler/src/executor.rs crates/aspen-forge-protocol/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-1-1-6-openspec-preflight.txt`

### `cargo test -p aspen-forge jj::`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-1-aspen-forge-jj-object-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs crates/aspen-forge/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-1-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-1-openspec-preflight.txt`

### `cargo test -p aspen-forge jj::`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-2-aspen-forge-jj-store-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs crates/aspen-forge/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-2-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-2-openspec-preflight.txt`

### `cargo test -p aspen-forge jj::`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-4-aspen-forge-jj-index-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs crates/aspen-forge/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-4-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-4-openspec-preflight.txt`

### `cargo test -p aspen-forge jj::`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-7-aspen-forge-jj-conflict-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs crates/aspen-forge/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-7-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-7-openspec-preflight.txt`

### `cargo test -p aspen-forge jj::`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-6-aspen-forge-jj-graph-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs crates/aspen-forge/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-6-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-6-openspec-preflight.txt`

### `cargo test -p aspen-forge jj::`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-5-aspen-forge-jj-staged-publish-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs crates/aspen-forge/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-5-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-5-openspec-preflight.txt`

### `cargo test -p aspen-forge-handler active_backend_routes`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-7-1-8-forge-handler-route-tests.txt`

### `rustfmt --check crates/aspen-forge-handler/src/executor.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-7-1-8-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-7-1-8-openspec-preflight.txt`

### `cargo test -p aspen-forge jj::`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-8-aspen-forge-jj-staged-cleanup-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs crates/aspen-forge/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-8-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-8-openspec-preflight.txt`

### `cargo test -p aspen-forge jj::`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-9-aspen-forge-jj-repo-state-tests.txt`

### `rustfmt --check crates/aspen-forge/src/jj.rs crates/aspen-forge/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-9-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-9-openspec-preflight.txt`

### `cargo test -p aspen-forge-protocol jj_native && cargo test -p aspen-forge-handler jj_native_admission && cargo test -p aspen-client-api request_metadata && cargo test -p aspen-client-api client_rpc_postcard_baseline`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-jj-native-admission-tests.txt`

### `rustfmt --check --config skip_children=true crates/aspen-forge-protocol/src/lib.rs crates/aspen-client-api/src/messages/mod.rs crates/aspen-client-api/src/messages/request_metadata.rs crates/aspen-client-api/src/messages/request_metadata_apps/forge.rs crates/aspen-forge-handler/src/executor.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-openspec-preflight.txt`

## Notes

- Task 1.4 remains unchecked: protocol structs exist, but a JJ session admission path still needs to enforce version compatibility before object exchange.
- Task 1.5 remains unchecked: manifests can claim protocols, but bounded protocol-session routing/admission still needs runtime implementation.
- Task 1.7 remains unchecked until JJ routing identifiers are preserved through reload/upgrade evidence.
- Same-family review correction is tracked in `openspec/changes/jj-native-forge-wasm-plugin/evidence/review-remediation-task-scope.md`.
