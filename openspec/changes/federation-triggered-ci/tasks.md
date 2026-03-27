## 1. Gossip emission from federation handlers

- [x] 1.1 Add `announce_ref_update(repo_id, ref_name, new_hash, old_hash)` method to `ForgeNode`
- [x] 1.2 In `update_mirror_refs()`, call `announce_ref_update()` for each successfully updated ref
- [x] 1.3 Wire gossip emission into `handle_federation_fetch_refs` after `update_mirror_refs` succeeds
- [x] 1.4 Wire gossip emission into `handle_federation_bidi_sync` pull path after mirror ref updates
- [x] 1.5 Wire gossip emission into `handle_federation_pull_remote` path
- [x] 1.6 Unit test: verify `announce_ref_update` is a no-op when gossip is None

## 2. TriggerService auto-watch for mirrors

- [x] 2.1 Add `federation_ci_enabled: bool` field to `TriggerServiceConfig` (default `false`)
- [x] 2.2 Add `scan_and_watch_mirrors(kv: &dyn KeyValueStore)` method that scans `_fed:mirror:*` and calls `watch_repo()`
- [x] 2.3 Spawn periodic mirror scan task (every 60s) via `start_mirror_scan_task()`
- [x] 2.4 In `CiTriggerHandler::on_announcement`, skip federation mirror repos when `federation_ci_enabled` is false
- [x] 2.5 Unit test: verify mirror scan discovers and watches mirror repos
- [x] 2.6 Unit test: verify announcements for mirrors are dropped when federation_ci_enabled is false

## 3. Node startup wiring

- [x] 3.1 Pass KV store reference to TriggerService for mirror scanning
- [x] 3.2 Call `scan_and_watch_mirrors()` during node startup when both `forge` and `ci` features are enabled
- [x] 3.3 Verify `ForgeConfigFetcher` can read `.aspen/ci.ncl` from mirror repo commit trees (existing code path — verified: uses same blob store as federation_import_objects)

## 4. NixOS VM integration test

- [ ] 4.1 Create `nix/tests/federation-ci-trigger-test.nix` with two VMs (alice, bob)
- [ ] 4.2 Bob: start node, create repo with `.aspen/ci.ncl` containing a shell job, push a commit
- [ ] 4.3 Alice: start node with `federation_ci_enabled = true`, federation-pull from Bob
- [ ] 4.4 Assert: Alice's `ci runs` shows a pipeline run for the mirror repo
- [ ] 4.5 Register test in `flake.nix` checks
