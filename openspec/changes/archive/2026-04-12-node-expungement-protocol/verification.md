# Verification Evidence

## Implementation Evidence

- Changed file: `openspec/changes/archive/2026-04-12-node-expungement-protocol/verification.md`
- Changed file: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/archive-status.txt`
- Changed file: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-cluster-bootstrap-peer-probe-test.txt`
- Changed file: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-cluster-handler-expunge-tests.txt`
- Changed file: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-raft-expungement-tests.txt`
- Changed file: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-raft-peer-expungement-probe-test.txt`
- Changed file: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-raft-trust-share-client-test.txt`
- Changed file: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-rpc-handlers-bootstrap-tests.txt`
- Changed file: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-trust-expunged-tests.txt`
- Changed file: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/bash-n-openspec-preflight.txt`
- Changed file: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`
- Changed file: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation-history.txt`
- Changed file: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/openspec-preflight.txt`
- Changed file: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`
- Changed file: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/trust-expungement-vm-test.txt`
- Changed file: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/trust-expungement-vm-test-full.log`

## Task Coverage

- [x] 1.1 Add `ExpungedMetadata { epoch: u64, removed_by: u64 }` to `aspen-core` types (serializable, stored in redb)
  - Evidence: `crates/aspen-cluster-types/src/lib.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`
- [x] 1.2 Add `trust_expunged` redb table: `TableDefinition<(), &[u8]>` (singleton — either empty or contains serialized `ExpungedMetadata`)
  - Evidence: `crates/aspen-raft/src/storage_shared/types.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`
- [x] 1.3 Implement `is_expunged() -> bool` and `load_expunged() -> Option<ExpungedMetadata>` on `RedbStorage`
  - Evidence: `crates/aspen-raft/src/storage_shared/trust.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`
- [x] 1.4 Implement `mark_expunged(metadata: ExpungedMetadata)` on `RedbStorage` — writes metadata and zeroizes all `trust_shares` entries
  - Evidence: `crates/aspen-raft/src/storage_shared/trust.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-raft-expungement-tests.txt`

- [x] 2.1 Add `Expunged(epoch: u64)` variant to trust protocol message enum
  - Evidence: `crates/aspen-trust/src/protocol.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-trust-expunged-tests.txt`
- [x] 2.2 In every trust message handler (`handle_get_share`, `handle_prepare`, etc.): add `is_expunged()` check as first line, drop message and log if true
  - Evidence: `crates/aspen-transport/src/trust.rs`, `crates/aspen-raft/src/node/trust.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`
- [x] 2.3 In `handle_get_share`: if requester is not in current configuration members, respond with `Expunged(current_epoch)` instead of the share
  - Evidence: `crates/aspen-raft/src/node/trust.rs`, `crates/aspen-raft/src/trust_share_client.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`
- [x] 2.4 In `handle_expunged(from, epoch)`: validate epoch is >= our latest known config epoch, then call `mark_expunged()`
  - Evidence: `crates/aspen-transport/src/trust.rs`, `crates/aspen-raft/src/node/trust.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`

- [x] 3.1 Add expungement check to Raft RPC handlers: reject `AppendEntries`, `RequestVote`, `InstallSnapshot` if `is_expunged()` is true
  - Evidence: `crates/aspen-transport/src/raft_authenticated.rs`, `crates/aspen-raft/src/server.rs`, `crates/aspen-raft/src/node/mod.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`
- [x] 3.2 On membership removal (via `change_membership`): if trust feature is enabled, send `Expunged(new_epoch)` to removed node via Iroh
  - Evidence: `crates/aspen-raft/src/node/trust.rs`, `crates/aspen-raft/src/trust_share_client.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-raft-trust-share-client-test.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/trust-expungement-vm-test-full.log`
- [x] 3.3 On node startup: check `is_expunged()` before initializing Raft — if expunged, log error and refuse to start (exit with clear message)
  - Evidence: `crates/aspen-cluster/src/bootstrap/node/storage_init.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/trust-expungement-vm-test-full.log`

- [x] 4.1 Implement secure share deletion: overwrite each share value in `trust_shares` with zeros before deleting the key
  - Evidence: `crates/aspen-raft/src/storage_shared/trust.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`
- [x] 4.2 Call `redb::Table::remove()` after overwrite to reclaim space
  - Evidence: `crates/aspen-raft/src/storage_shared/trust.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`
- [x] 4.3 Test: after expungement, `load_share()` returns `None` for all epochs
  - Evidence: `crates/aspen-raft/src/storage_shared/trust.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-raft-expungement-tests.txt`

- [x] 5.1 Add `aspen-cli cluster expunge <node-id>` command with `--confirm` required flag
  - Evidence: `crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs`, `crates/aspen-client-api/src/messages/cluster.rs`, `crates/aspen-client-api/src/messages/mod.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-cluster-handler-expunge-tests.txt`
- [x] 5.2 Print warning message before execution: "This will permanently remove node {id} from the cluster. The node will need a factory reset to rejoin."
  - Evidence: `crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`
- [x] 5.3 Implement RPC: send expunge request to Raft leader, which triggers membership removal + trust reconfiguration + expungement notification
  - Evidence: `crates/aspen-client-api/src/messages/cluster.rs`, `crates/aspen-client-api/src/messages/mod.rs`, `crates/aspen-cluster-handler/src/handler/mod.rs`, `crates/aspen-cluster-handler/src/handler/membership.rs`, `crates/aspen-raft/src/node/trust.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`
- [x] 5.4 Report success/failure to the operator
  - Evidence: `crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs`, `crates/aspen-cluster-handler/src/handler/membership.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`

- [x] 6.1 Unit test: `mark_expunged()` sets the flag, zeroizes shares, survives reload
  - Evidence: `crates/aspen-raft/src/storage_shared/trust.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-raft-expungement-tests.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`
- [x] 6.2 Unit test: expunged node rejects all trust protocol messages
  - Evidence: `crates/aspen-transport/src/trust.rs`, `crates/aspen-raft/src/node/trust.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`
- [x] 6.3 Unit test: peer sends `Expunged` when receiving GetShare from non-member
  - Evidence: `crates/aspen-raft/src/node/trust.rs`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-raft-trust-share-client-test.txt`
- [x] 6.4 Integration test: 3-node cluster → expunge node 3 → verify node 3 can't rejoin → wipe data_dir → re-add as fresh member
  - Evidence: `nix/tests/trust-expungement.nix`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/trust-expungement-vm-test.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/trust-expungement-vm-test-full.log`
- [x] 6.5 Test: expunged node that never received the message gets expunged via peer enforcement on next communication attempt
  - Evidence: `crates/aspen-raft/src/node/trust.rs`, `crates/aspen-raft/src/trust_peer_probe.rs`, `crates/aspen-cluster/src/bootstrap/node/mod.rs`, `nix/tests/trust-expungement.nix`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/source-locations.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-raft-peer-expungement-probe-test.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-cluster-bootstrap-peer-probe-test.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/trust-expungement-vm-test.txt`, `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/trust-expungement-vm-test-full.log`

## Review Scope Snapshot

### `git diff 70840a989^ -- crates/aspen-cluster-types/src/lib.rs crates/aspen-trust/src/protocol.rs crates/aspen-transport/src/trust.rs crates/aspen-transport/src/raft_authenticated.rs crates/aspen-raft/src/server.rs crates/aspen-raft/src/node/mod.rs crates/aspen-raft/src/node/trust.rs crates/aspen-raft/src/trust_share_client.rs crates/aspen-raft/src/storage_shared/types.rs crates/aspen-raft/src/storage_shared/trust.rs crates/aspen-cluster/src/bootstrap/node/mod.rs crates/aspen-cluster/src/bootstrap/node/storage_init.rs crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs crates/aspen-client-api/src/messages/cluster.rs crates/aspen-client-api/src/messages/mod.rs crates/aspen-cluster-handler/src/handler/mod.rs crates/aspen-cluster-handler/src/handler/membership.rs nix/tests/trust-expungement.nix openspec/changes/archive/2026-04-12-node-expungement-protocol/tasks.md openspec/changes/archive/2026-04-12-node-expungement-protocol/specs/node-expungement/spec.md`

- Status: captured
- Artifact: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation.diff`

## Verification Commands

### `cargo test -p aspen-raft --features trust,testing expung -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-raft-expungement-tests.txt`

### `cargo test -p aspen-trust expunged -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-trust-expunged-tests.txt`

### `cargo test -p aspen-cluster --features blob,docs,jobs,hooks,trust test_create_raft_node_spawns_trust_peer_probe_during_bootstrap -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-cluster-bootstrap-peer-probe-test.txt`

### `cargo test -p aspen-cluster-handler expunge -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-cluster-handler-expunge-tests.txt`

### `cargo test -p aspen-raft --features trust,testing trust_share_client -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-raft-trust-share-client-test.txt`

### `cargo test -p aspen-raft --features trust,testing spawn_trust_peer_probe_marks_removed_node -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-raft-peer-expungement-probe-test.txt`

### `cargo test -p aspen-rpc-handlers --lib -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-rpc-handlers-bootstrap-tests.txt`

### `nix build .#checks.x86_64-linux.trust-expungement-test --option sandbox false`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/trust-expungement-vm-test.txt`
- Artifact: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/trust-expungement-vm-test-full.log`

### `bash -n scripts/openspec-preflight.sh`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/bash-n-openspec-preflight.txt`

### `scripts/openspec-preflight.sh openspec/changes/archive/2026-04-12-node-expungement-protocol`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/openspec-preflight.txt`

## Notes

- Commit history for the implementation is captured in `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/implementation-history.txt`; the main code landed across `70840a989` (`Implement node expungement core types and storage`), `c4eb0d86b` (`feat(trust): implement node expungement protocol`), and `05eaf75f4` (`Add NixOS VM integration test for node expungement`).
- Follow-up verification work fixed three gaps the original transcript missed: trust bootstrap requests are now classified as bootstrap RPCs, direct expungement now waits for an explicit peer acknowledgement after `on_expunged()` finishes, and startup now probes peers for `Expunged` when a node comes back with stale trust metadata.
- `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-raft-trust-share-client-test.txt` covers the strict return-time contract by blocking `on_expunged()` on the peer and asserting `send_expunged().await` does not complete until that peer-side processing is released.
- `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-raft-peer-expungement-probe-test.txt` exercises `spawn_trust_peer_probe()` itself rather than calling `probe_for_peer_expungement()` directly.
- `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/aspen-cluster-bootstrap-peer-probe-test.txt` covers the bootstrap wiring in `crates/aspen-cluster/src/bootstrap/node/mod.rs` by proving `create_raft_node()` starts the peer probe during node creation.
- `openspec/changes/archive/2026-04-12-node-expungement-protocol/evidence/archive-status.txt` records that `openspec/changes/node-expungement-protocol` no longer exists after archiving.
