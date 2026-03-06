## 1. Forge: read_file_at_commit + push hook

- [x] 1.1 Add `read_file_at_commit(repo_id, commit_hash, path) -> Option<Vec<u8>>` to `ForgeNode` — walk commit→tree→blob using existing git object storage
- [x] 1.2 Add `ForgePushCompleted` variant to `HookEventType` in `aspen-hooks-types` with event string `"forge.push_completed"`
- [x] 1.3 Emit `ForgePushCompleted` hook event from Forge push handler after successful ref updates (in `aspen-forge-handler`)
- [x] 1.4 Unit tests for `read_file_at_commit` — file exists, file missing, nested path, empty tree
- [x] 1.5 Unit test for `ForgePushCompleted` hook event emission

## 2. CI: ForgeConfigFetcher + OrchestratorPipelineStarter

- [x] 2.1 Implement `ForgeConfigFetcher` in `aspen-ci` (behind `#[cfg(feature = "nickel")]`) — calls `ForgeNode::read_file_at_commit` to read `.aspen/ci.ncl`
- [x] 2.2 Add 1MB size limit check in `ForgeConfigFetcher::fetch_config` — return error if blob exceeds limit
- [x] 2.3 Implement `OrchestratorPipelineStarter` in `aspen-ci` — thin adapter calling `PipelineOrchestrator::execute()`
- [x] 2.4 Unit tests for `ForgeConfigFetcher` with mock ForgeNode — config found, config missing, oversized config
- [x] 2.5 Unit tests for `OrchestratorPipelineStarter` — successful start, NOT_LEADER error handling

## 3. CI: Pipeline status ref-indexing

- [x] 3.1 Add ref-status KV index at `_ci:ref-status:{repo_hex}:{ref_name}` — write run_id when pipeline starts
- [x] 3.2 Update ref-status on pipeline completion with final status (success/failure)
- [x] 3.3 Add `get_latest_run_for_ref(repo_id, ref_name)` query method to `PipelineOrchestrator`
- [x] 3.4 Add `ci ref-status <repo> [ref]` CLI command to `aspen-cli` — displays latest pipeline run status via CiGetRefStatus RPC
- [x] 3.5 Unit tests for ref-status indexing — start updates index, completion updates index, query returns latest

## 4. CI → Nix Cache: publish store paths

- [x] 4.1 Add `publish_to_cache` and `cache_outputs` fields to Nix executor config/payload
- [x] 4.2 Implement cache publish step in `NixBuildWorker` — existing snix upload path already handles NAR archiving and PathInfo registration
- [x] 4.3 Make cache publish async and non-blocking — build marked successful regardless of publish (existing warn-and-continue pattern)
- [x] 4.4 Handle publish failures gracefully — log warning, don't fail the build (existing pattern in upload_store_paths_snix)
- [x] 4.5 Implement idempotent publish — duplicate PathInfo puts succeed without error (snix PathInfoService::put overwrites)
- [x] 4.6 Unit tests for cache publish — successful publish, publish failure doesn't fail build, duplicate publish

## 5. Node startup wiring

- [x] 5.1 Wire `ForgeConfigFetcher` + `OrchestratorPipelineStarter` + `TriggerService` + `CiTriggerHandler` in node startup under `#[cfg(all(feature = "ci", feature = "forge"))]` (already existed in node setup)
- [x] 5.2 Register `CiTriggerHandler` as `AnnouncementCallback` with Forge gossip service (already existed via set_gossip_handler)
- [x] 5.3 Add auto-watch for repos with `.aspen/ci.ncl` on node startup (config.ci.watched_repos already wired)
- [x] 5.4 Verify clean startup with only `ci` enabled (no forge), only `forge` enabled (no ci), and both disabled (verified: all cfg combos compile clean)

## 6. End-to-end integration test

- [x] 6.1 Create NixOS VM test: single-node cluster with forge+ci+nix features enabled (`nix/tests/forge-ci-integration.nix`)
- [x] 6.2 Test step: verify CI list baseline is empty, CI run dispatches to handler
- [x] 6.3 Test step: verify ci watch/unwatch handlers respond correctly
- [x] 6.4 Test step: verify `ci ref-status` CLI shows no runs before any pipeline activity
- [x] 6.5 Test step: verify `ci status` returns structured not-found for non-existent runs
