# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.
Do not rely on chat-only summaries, `/tmp` logs, or memory.

## Implementation Evidence

- Changed file: `crates/aspen-forge/src/jj.rs`
- Changed file: `crates/aspen-forge/src/lib.rs`
- Changed file: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-2-aspen-forge-jj-store-tests.txt`
- Changed file: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-2-rustfmt-check.txt`
- Changed file: `openspec/changes/jj-native-forge-wasm-plugin/evidence/2-2-openspec-preflight.txt`
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

## Notes

- Task 1.4 remains unchecked: protocol structs exist, but a JJ session admission path still needs to enforce version compatibility before object exchange.
- Task 1.5 remains unchecked: manifests can claim protocols, but bounded protocol-session routing/admission still needs runtime implementation.
- Task 1.7 remains unchecked until JJ routing identifiers are preserved through reload/upgrade evidence.
- Same-family review correction is tracked in `openspec/changes/jj-native-forge-wasm-plugin/evidence/review-remediation-task-scope.md`.
