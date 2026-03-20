## 1. Discussion COB Types & Operations

- [x] 1.1 Add `CobOperation` variants to `cob/change.rs`: `CreateDiscussion { title, body, labels }`, `Reply { body, parent_reply: Option<[u8; 32]> }`, `ResolveThread { reply_hash: [u8; 32] }`, `UnresolveThread { reply_hash: [u8; 32] }`, `LockDiscussion`, `UnlockDiscussion`
- [x] 1.2 Create `cob/discussion.rs` with `Discussion` struct (title, body, state, replies, labels, reactions, resolved_threads, created_at_ms, updated_at_ms), `DiscussionState` enum (Open, Closed, Locked), `DiscussionReply` struct (author, body, parent_reply, timestamp_ms, change_hash)
- [x] 1.3 Implement `Discussion::apply_change` handling all discussion-relevant CobOperation variants (create, reply, resolve/unresolve thread, lock/unlock, close/reopen, label, react, edit title/body)
- [x] 1.4 Add unit tests for `Discussion::apply_change` covering creation, replies, thread resolution, locking, close/reopen lifecycle

## 2. Discussion COB Store Operations

- [x] 2.1 Create `cob/store/discussion_ops.rs` with `create_discussion`, `add_reply`, `resolve_thread`, `unresolve_thread`, `lock_discussion`, `unlock_discussion`, `close_discussion`, `reopen_discussion`
- [x] 2.2 Implement `resolve_discussion` in `discussion_ops.rs` following the same DAG walk + topological sort + field resolution pattern as `resolve_issue`
- [x] 2.3 Implement `list_discussions` with state filtering (Open, Closed, Locked, All) and limit parameter
- [x] 2.4 Add `discussion_ops` module to `cob/store/mod.rs` and re-export `Discussion`, `DiscussionState`, `DiscussionReply` from `cob/mod.rs` and `lib.rs`
- [x] 2.5 Add validation: reject `Reply` on locked discussions with `ForgeError::DiscussionLocked`

## 3. Discussion RPC & CLI

- [x] 3.1 Add RPC request/response types to `aspen-client-api`: `ForgeCreateDiscussion`, `ForgeListDiscussions`, `ForgeShowDiscussion`, `ForgeReplyDiscussion`, `ForgeLockDiscussion`, `ForgeCloseDiscussion`
- [x] 3.2 Add handler implementations in `aspen-rpc-handlers` for discussion RPCs
- [x] 3.3 Create `aspen-cli/src/bin/aspen-cli/commands/discussion.rs` with subcommands: `create`, `list`, `show`, `reply`, `close`, `lock`
- [x] 3.4 Register `Discussion` command in CLI dispatch (`cli.rs`)

## 4. Fork Support

- [x] 4.1 Add `ForkInfo` struct to `identity/mod.rs` with `upstream_repo_id: RepoId` and `upstream_cluster: Option<PublicKey>`
- [x] 4.2 Add `fork_info: Option<ForkInfo>` field to `RepoIdentity` (default None, set at fork time)
- [x] 4.3 Implement `ForgeNode::fork_repo(upstream_repo_id, name, delegates, threshold)` that creates a new repo with ForkInfo, copies all refs from upstream, and starts seeding
- [x] 4.4 Add RPC types `ForgeForkRepo` request/response to `aspen-client-api`
- [x] 4.5 Add fork handler to `aspen-rpc-handlers`
- [x] 4.6 Add `aspen-cli repo fork --upstream U --name N` command
- [x] 4.7 Add unit tests for fork_repo: basic fork, fork-of-fork lineage, fork non-existent repo error

## 5. Mirror Support

- [x] 5.1 Add `MirrorConfig` struct (upstream_repo_id, upstream_cluster, interval_secs, last_sync_ms, enabled) and `MirrorStatus` struct
- [x] 5.2 Add KV persistence for MirrorConfig at `forge:mirror:{repo_id}` with set/get/delete methods on ForgeNode
- [x] 5.3 Implement mirror sync as `sync_mirror_refs` pure function + `mirror_sync_job_spec` for aspen-jobs recurring scheduling (uses Schedule::interval + idempotency_key, tested with 5 mirror sync tests)
- [x] 5.4 Add Tiger Style bounds: max 1000 mirrors per node, interval clamped to 60–3600s, max 10 concurrent syncs
- [x] 5.5 Add RPC types `ForgeSetMirror`, `ForgeDisableMirror`, `ForgeGetMirrorStatus` to `aspen-client-api`
- [x] 5.6 Add mirror handlers to `aspen-rpc-handlers`
- [x] 5.7 Add `aspen-cli repo mirror --repo R --upstream U --interval 300`, `--disable`, `--status` commands
- [x] 5.8 Add unit tests for MirrorConfig persistence and interval clamping

## 6. Integration Tests

- [x] 6.1 Create `tests/forge_lifecycle_test.rs` with repo create → blob → tree → commit → push → get_head roundtrip test
- [x] 6.2 Add issue lifecycle test: create → comment → label → close → reopen → resolve
- [x] 6.3 Add patch lifecycle test: create repo → init commit → create patch → approve → merge → verify ref and state
- [x] 6.4 Add discussion lifecycle test: create → reply (top-level + threaded) → resolve thread → close → resolve
- [x] 6.5 Add fork integration test: create upstream with commits → fork → verify refs match and fork_info present
- [x] 6.6 Add multi-COB test: create repo with issue, patch, and discussion; verify all three resolve independently

## 7. Property-Based Tests

- [x] 7.1 Create `tests/proptest_cob_resolution.rs` with proptest `Arbitrary` impl for `CobOperation` (issue variants only)
- [x] 7.2 Add idempotency property test: apply random issue operation sequence twice, assert same resolved state
- [x] 7.3 Add label convergence property test: two independent AddLabel ops in both orderings produce same label set
- [x] 7.4 Add comment convergence property test: independent Comment ops produce same body set regardless of order
- [x] 7.5 Add DAG well-formedness property test: generated DAGs pass topological sort with no cycles and exactly one root
- [x] 7.6 Add discussion operation generators and idempotency property test for Discussion resolution
- [x] 7.7 Gate property tests behind `#[cfg(not(feature = "quick"))]` and add to `ci` nextest profile

## 8. Documentation & Spec Updates

- [x] 8.1 Update `docs/forge.md` with Discussion COB section, fork/mirror documentation, and updated implementation progress
- [x] 8.2 Add `ForgeError` variants: `DiscussionLocked`, `MirrorLimitReached`, `MirrorIntervalOutOfRange`
- [x] 8.3 Update `constants.rs` with `MAX_MIRRORS_PER_NODE`, `MIN_MIRROR_INTERVAL_SECS`, `MAX_MIRROR_INTERVAL_SECS`, `MAX_CONCURRENT_MIRROR_SYNCS`, `MAX_DISCUSSION_REPLY_BODY_BYTES`
