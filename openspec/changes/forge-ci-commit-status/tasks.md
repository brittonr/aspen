## 1. Commit Status Types (aspen-forge)

- [x] 1.1 Create `crates/aspen-forge/src/status/mod.rs` with `CommitCheckState` enum (Pending, Success, Failure, Error) and `CommitStatus` struct (repo_id, commit, context, state, description, pipeline_run_id, created_at_ms)
- [x] 1.2 Create `crates/aspen-forge/src/status/store.rs` with `StatusStore<K: KeyValueStore>` — methods: `set_status`, `get_status`, `list_statuses_for_commit` (scan `forge:status:{repo_hex}:{commit_hex}:` prefix)
- [x] 1.3 Add `pub mod status` to `crates/aspen-forge/src/lib.rs` and re-export types
- [x] 1.4 Add KV prefix constants to `crates/aspen-forge/src/constants.rs`: `KV_PREFIX_COMMIT_STATUS = "forge:status:"` and `KV_PREFIX_BRANCH_PROTECTION = "forge:protection:"`
- [x] 1.5 Write unit tests for `StatusStore`: set/get/overwrite/list/empty cases

## 2. Pipeline Status Gossip Announcement (aspen-forge)

- [x] 2.1 Add `PipelineStatus { repo_id, commit, ref_name, run_id, status: CommitCheckState, context: String }` variant to `Announcement` enum in `crates/aspen-forge/src/gossip/types.rs`
- [x] 2.2 Update `Announcement::repo_id()` match arm for the new variant
- [x] 2.3 Add roundtrip serialization test and sign/verify test for the new variant in `gossip/types.rs` tests

## 3. Status Reporter Trait (aspen-ci)

- [x] 3.1 Create `crates/aspen-ci/src/status_reporter.rs` with `CommitStatusReport` struct (repo_id, commit_hash, context, state: CommitCheckState, description, pipeline_run_id) and `StatusReporter` async trait with `report_status` method
- [x] 3.2 Add `status_reporter: Option<Arc<dyn StatusReporter>>` field to `PipelineOrchestrator` struct in `orchestrator/pipeline/mod.rs`
- [x] 3.3 Update `PipelineOrchestrator::new` to accept an optional `StatusReporter`
- [x] 3.4 Add `pub mod status_reporter` to `crates/aspen-ci/src/lib.rs` and re-export types

## 4. Report Status on Pipeline Transitions (aspen-ci)

- [x] 4.1 In `orchestrator/pipeline/persistence.rs` `track_run`, call `report_status` with `Pending` state after persisting the run. Log and swallow errors.
- [x] 4.2 In `orchestrator/pipeline/status.rs` `finalize_status_sync`, call `report_status` with the terminal state (map `PipelineStatus::Success` → `CommitCheckState::Success`, `Failed` → `Failure`, `Cancelled` → `Error`). Log and swallow errors.
- [x] 4.3 Add a helper `pipeline_status_to_check_state(PipelineStatus) -> CommitCheckState` in `status_reporter.rs`
- [x] 4.4 Write tests with a mock `StatusReporter` that records calls, verifying report is called on create and on terminal transition

## 5. Forge Status Reporter Implementation (aspen-ci)

- [x] 5.1 Create `ForgeStatusReporter` struct in `crates/aspen-ci/src/status_reporter.rs` (or a separate `forge_reporter.rs`) gated behind `cfg(feature = "forge")`. Holds `Arc<K: KeyValueStore>` and `Option<Arc<ForgeGossipService>>`
- [x] 5.2 Implement `StatusReporter` for `ForgeStatusReporter`: write `CommitStatus` JSON to `forge:status:{repo_hex}:{commit_hex}:{context}` via `KeyValueStore::write`
- [x] 5.3 In the same `report_status` impl, if gossip service is present, broadcast `Announcement::PipelineStatus` signed with the node's secret key
- [x] 5.4 Write integration test: create `ForgeStatusReporter` with an in-memory KV store, call `report_status`, verify KV entry written with correct key and JSON contents

## 6. Branch Protection Types (aspen-forge)

- [x] 6.1 Create `crates/aspen-forge/src/protection/mod.rs` with `BranchProtection` struct (ref_pattern, required_approvals: u32, required_ci_contexts: Vec<String>, dismiss_stale_approvals: bool)
- [x] 6.2 Add `ProtectionStore<K: KeyValueStore>` with methods: `set_rule`, `get_rule`, `delete_rule`, `list_rules` — keyed at `forge:protection:{repo_hex}:{ref_pattern}`
- [x] 6.3 Add `pub mod protection` to `crates/aspen-forge/src/lib.rs` and re-export types
- [x] 6.4 Write unit tests for `ProtectionStore`: set/get/delete/list

## 7. Merge Gating Enforcement (aspen-forge)

- [x] 7.1 Add `check_merge_allowed` method to `CobStore` (or a new `MergeChecker` struct) that takes repo_id, target ref, patch head commit, and patch approvals. Returns `Result<(), ForgeError>` with a new `MergeBlocked` error variant.
- [x] 7.2 In `check_merge_allowed`: load `BranchProtection` for the target ref. If none, return Ok. If present, query `StatusStore::list_statuses_for_commit` for the head commit and verify all `required_ci_contexts` have `Success` state.
- [x] 7.3 In `check_merge_allowed`: verify approval count meets `required_approvals`. If `dismiss_stale_approvals` is true, only count approvals whose `commit` matches the current head.
- [x] 7.4 Call `check_merge_allowed` from the code path that processes `CobOperation::Merge` — reject the operation if the check fails.
- [x] 7.5 Write tests: merge blocked by failing CI, merge blocked by missing status, merge blocked by insufficient approvals, merge allowed when all checks pass, unprotected ref allows unconditional merge, stale approvals dismissed

## 8. Patch CI Status Resolution (aspen-forge)

- [x] 8.1 Add `ci_statuses: Vec<CommitStatus>` field to `Patch` struct in `cob/patch.rs` (default empty)
- [x] 8.2 Add `resolve_ci_status` method (or extend existing resolution) that takes a `StatusStore` reference and populates `ci_statuses` by querying for the patch's `head` commit
- [x] 8.3 Write tests: patch with passing CI shows status, patch with no CI shows empty, patch with multiple contexts shows all

## 9. Hook Event Types (aspen-hooks)

- [x] 9.1 Add forge event variants to `HookEventType` enum: `RefUpdated`, `PatchCreated`, `PatchMerged`, `PatchClosed`, `PatchApproved`, `RepoCreated`
- [x] 9.2 Add CI event variants to `HookEventType` enum: `PipelineStarted`, `PipelineCompleted`, `PipelineFailed`, `DeployCompleted`
- [x] 9.3 Update `topic_for_event_type` mapping: forge events → `hooks.forge.*`, CI events → `hooks.ci.*`
- [x] 9.4 Write tests for topic mapping of all new event types

## 10. Node Startup Wiring

- [x] 10.1 In the node startup path (where `TriggerService` and `CiTriggerHandler` are created), also create `ForgeStatusReporter` with the KV store and gossip service when both `forge` and `ci` features are enabled
- [x] 10.2 Pass the `ForgeStatusReporter` to `PipelineOrchestrator::new`
- [x] 10.3 Verify that with only `ci` enabled (no `forge`), the orchestrator is constructed without a reporter and compiles cleanly

## 11. Integration Testing

- [x] 11.1 Write end-to-end test: push to forge repo → CI triggers → pipeline completes → commit status written to forge KV → status queryable via `StatusStore`
- [x] 11.2 Write end-to-end test: create patch → CI triggers on patch head → pipeline succeeds → merge allowed. Then: pipeline fails → merge blocked.
- [x] 11.3 Verify gossip announcement is received by a subscribed node when pipeline status changes
