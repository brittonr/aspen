## 1. Core Types and Persistent State

- [x] 1.1 Add `ExpungedMetadata { epoch: u64, removed_by: u64 }` to `aspen-core` types (serializable, stored in redb)
- [x] 1.2 Add `trust_expunged` redb table: `TableDefinition<(), &[u8]>` (singleton — either empty or contains serialized `ExpungedMetadata`)
- [x] 1.3 Implement `is_expunged() -> bool` and `load_expunged() -> Option<ExpungedMetadata>` on `RedbStorage`
- [x] 1.4 Implement `mark_expunged(metadata: ExpungedMetadata)` on `RedbStorage` — writes metadata and zeroizes all `trust_shares` entries

## 2. Trust Protocol Messages

- [x] 2.1 Add `Expunged(epoch: u64)` variant to trust protocol message enum
- [x] 2.2 In every trust message handler (`handle_get_share`, `handle_prepare`, etc.): add `is_expunged()` check as first line, drop message and log if true
- [x] 2.3 In `handle_get_share`: if requester is not in current configuration members, respond with `Expunged(current_epoch)` instead of the share
- [x] 2.4 In `handle_expunged(from, epoch)`: validate epoch is >= our latest known config epoch, then call `mark_expunged()`

## 3. Raft Integration

- [x] 3.1 Add expungement check to Raft RPC handlers: reject `AppendEntries`, `RequestVote`, `InstallSnapshot` if `is_expunged()` is true
- [x] 3.2 On membership removal (via `change_membership`): if trust feature is enabled, send `Expunged(new_epoch)` to removed node via Iroh
- [x] 3.3 On node startup: check `is_expunged()` before initializing Raft — if expunged, log error and refuse to start (exit with clear message)

## 4. Share Zeroization

- [x] 4.1 Implement secure share deletion: overwrite each share value in `trust_shares` with zeros before deleting the key
- [x] 4.2 Call `redb::Table::remove()` after overwrite to reclaim space
- [x] 4.3 Test: after expungement, `load_share()` returns `None` for all epochs

## 5. CLI Command

- [x] 5.1 Add `aspen-cli cluster expunge <node-id>` command with `--confirm` required flag
- [x] 5.2 Print warning message before execution: "This will permanently remove node {id} from the cluster. The node will need a factory reset to rejoin."
- [x] 5.3 Implement RPC: send expunge request to Raft leader, which triggers membership removal + trust reconfiguration + expungement notification
- [x] 5.4 Report success/failure to the operator

## 6. Testing

- [x] 6.1 Unit test: `mark_expunged()` sets the flag, zeroizes shares, survives reload
- [x] 6.2 Unit test: expunged node rejects all trust protocol messages
- [x] 6.3 Unit test: peer sends `Expunged` when receiving GetShare from non-member
- [x] 6.4 Integration test: 3-node cluster → expunge node 3 → verify node 3 can't rejoin → wipe data_dir → re-add as fresh member
- [x] 6.5 Test: expunged node that never received the message gets expunged via peer enforcement on next communication attempt
