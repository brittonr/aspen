# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.
Do not rely on chat-only summaries, `/tmp` logs, or memory.

## Implementation Evidence

- Changed file: `crates/aspen-forge-protocol/src/lib.rs`
- Changed file: `crates/aspen-plugin-api/src/manifest.rs`
- Changed file: `crates/aspen-plugin-api/src/resolve.rs`
- Changed file: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-protocol-plugin-api-tests.txt`
- Changed file: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-rustfmt-check.txt`
- Changed file: `openspec/changes/jj-native-forge-wasm-plugin/tasks.md`
- Changed file: `openspec/changes/jj-native-forge-wasm-plugin/verification.md`

## Task Coverage

- [x] 1.4 Add JJ-native request/response families, transport identifiers, and explicit transport-version advertisement/compatibility checks for clone, fetch, push, bookmark sync, and change-id lookup.
  - Evidence: `crates/aspen-forge-protocol/src/lib.rs`, `crates/aspen-plugin-api/src/manifest.rs`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-protocol-plugin-api-tests.txt`, `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-rustfmt-check.txt`

## Review Scope Snapshot

Pending broader implementation diff for later tasks.

## Verification Commands

### `cargo test -p aspen-forge-protocol -p aspen-plugin-api`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-protocol-plugin-api-tests.txt`

### `rustfmt --check crates/aspen-forge-protocol/src/lib.rs crates/aspen-plugin-api/src/manifest.rs crates/aspen-plugin-api/src/resolve.rs`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-rustfmt-check.txt`

### `scripts/openspec-preflight.sh jj-native-forge-wasm-plugin`

- Status: pass
- Artifact: `openspec/changes/jj-native-forge-wasm-plugin/evidence/1-4-openspec-preflight.txt`

## Notes

- Task 1.4 is the first protocol foundation slice only; runtime routing, Forge storage, and client helper tasks remain unchecked.
