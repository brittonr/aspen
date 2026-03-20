## Context

The Forge crate (~15k lines) implements decentralized Git hosting with COB-based collaboration. Issues, patches, and reviews each follow the same pattern: a struct with `apply_change`, resolution via DAG walk + topological sort in `CobStore`, and store-level operations (`create_*`, `close_*`, `resolve_*`). Discussion is declared as `CobType::Discussion` but has no resolver, struct, or store ops.

Repository identity (`RepoIdentity`) holds name, delegates, threshold, and default branch — no concept of upstream linkage. The `SyncService` fetches objects from peers but there's no structured fork or mirror workflow.

Integration tests cover DAG sync mechanics (`dag_sync_integration_test.rs`, `patchbay_dag_sync_test.rs`) but no test exercises the full lifecycle: repo creation → commit → push → issue/patch creation → merge → gossip announcement.

## Goals / Non-Goals

**Goals:**

- Discussion COB with full lifecycle (create, reply, resolve thread, lock) matching the existing Issue/Patch pattern
- `fork_repo` on ForgeNode creating a linked repository with shared object history
- Mirror mode: periodic ref sync from an upstream repo (same cluster or federated)
- End-to-end integration tests covering repo → commit → issue → patch → merge flows
- Property-based tests proving COB resolution invariants (idempotency, convergence)
- CLI and client RPC coverage for all new operations

**Non-Goals:**

- Web UI for discussions (TUI integration is a separate change)
- Cross-cluster federation protocol implementation (mirror uses existing SyncService)
- Shallow/partial clone support
- Discussion-to-issue or discussion-to-patch conversion (future work)
- GitHub/GitLab webhook-triggered mirroring (manual or cron-based for now)

## Decisions

### 1. Discussion COB follows the Issue/Patch pattern exactly

**Decision:** New `cob/discussion.rs` with `Discussion` struct, `DiscussionState`, `ThreadEntry`, and `apply_change`. New store ops in `cob/store/discussion_ops.rs`. New `CobOperation` variants: `CreateDiscussion`, `Reply`, `ResolveThread`, `LockDiscussion`, `UnlockDiscussion`.

**Why not a generic threaded-comment system?** Issues and patches already have comments, but discussions are standalone objects (not attached to a code change). Keeping them as a separate COB type with their own state machine is consistent with the existing architecture and avoids overloading the Issue/Patch comment fields.

**Alternative considered:** Reuse Issue with a `discussion` label. Rejected because discussions have different semantics — they don't have open/closed lifecycle tied to code changes, they have thread resolution, and they should be listable independently.

### 2. ForkInfo as optional field on RepoIdentity

**Decision:** Add `fork_info: Option<ForkInfo>` to `RepoIdentity` where `ForkInfo` contains `upstream_repo_id: RepoId` and `upstream_cluster: Option<PublicKey>` (None = same cluster). The `fork_repo` method on `ForgeNode` creates a new repo, copies all current refs, adds the caller as delegate, and records the fork relationship.

**Why on RepoIdentity?** Fork info is immutable metadata about the repo's origin — it belongs with the identity document, not as a separate KV entry. It's set once at fork time and never changes.

**Alternative considered:** Separate `forks:` KV prefix. Rejected because it would require an extra lookup on every repo display and complicates the identity verification chain.

### 3. Mirror as a background sync task, not a separate repo type

**Decision:** Add `MirrorConfig` stored at `forge:mirror:{repo_id}` in KV. A `MirrorWorker` polls upstream refs at a configurable interval (default: 5 minutes, bounded 1min–60min per Tiger Style). Mirror updates refs via the normal Raft path — no special bypass.

**Why polling instead of push notifications?** Polling is simpler, works across federation boundaries, and doesn't require the upstream to know about the mirror. Push can be added later as an optimization.

**Alternative considered:** Use iroh-gossip subscriptions for real-time mirror sync. Rejected for initial implementation because gossip requires both ends to be online and subscribed. Polling works for cold starts and federated remotes.

### 4. Discussion threads are flat with optional parent references

**Decision:** Each reply in a discussion references the discussion COB ID and optionally a `parent_reply: Option<[u8; 32]>` (hash of the reply being responded to). Resolution builds a flat list sorted by HLC timestamp. Clients can reconstruct thread hierarchy from parent references.

**Why not enforce tree structure in the COB?** The COB DAG already provides causal ordering. Adding tree structure inside the operations would duplicate what the DAG provides. Flat storage with optional parent refs gives clients flexibility to render threads or flat views.

### 5. Property tests use proptest with custom CobChange generators

**Decision:** Create proptest `Arbitrary` implementations for `CobOperation` and `CobChange`. Property tests verify:

- Idempotency: applying the same change twice produces the same state
- Convergence: two different orderings of independent changes produce the same final state
- Monotonicity: timestamps only increase, comment counts only grow

**Why proptest over bolero?** The existing codebase uses proptest extensively. Bolero can be added later via the `bolero` feature flag.

## Risks / Trade-offs

- **ForkInfo increases RepoIdentity size** → Mitigated by Option wrapping; only present on forked repos. Serialization overhead is negligible (one hash + one optional public key).
- **Mirror polling creates background load** → Tiger Style bounded: max 1000 mirrored repos per node, min 60s poll interval. MirrorWorker uses a single tokio task with a timer wheel, not one task per repo.
- **Discussion COB adds new CobOperation variants** → This is a backwards-incompatible change to the serialized format. Acceptable per AGENTS.md ("backwards compatibility is not a concern").
- **Property tests may be slow** → Use `#[cfg(not(feature = "quick"))]` and put them in the `ci` nextest profile with extended timeouts. Keep proptest case count at 256 (not 1000).

## Open Questions

- Should mirror mode support bidirectional sync (push local changes upstream), or read-only only for v1?
- Should discussions support pinning (pinned discussions appear first in listings)?
