## Why

CI pipeline results are invisible to Forge. When CI triggers from a `RefUpdate` gossip announcement and runs a pipeline, the result stays trapped inside CI's KV namespace. Forge patches show approvals and change requests but have no CI signal. There's no way to gate merges on passing CI. The feedback loop is broken — Forge talks to CI but CI never talks back.

## What Changes

- Forge gains a `CommitStatus` store that tracks external check results (pending/success/failure/error) keyed by repo + commit + context string
- CI writes commit statuses back to Forge when pipeline state changes, closing the feedback loop
- A new `PipelineStatus` gossip announcement broadcasts CI results in real-time so TUI/CLI/hooks can react without polling
- Patches resolve CI status alongside approvals, giving reviewers a single view of readiness
- Branch protection rules can require passing CI contexts before a `Merge` COB operation succeeds
- The hooks system gains Forge and CI event types for user-defined automation

## Capabilities

### New Capabilities

- `commit-status`: Commit status store in Forge, status reporter trait in CI, and gossip announcement for pipeline results
- `merge-gating`: Branch protection rules with required CI contexts and approval counts, enforced at merge time
- `forge-ci-hooks`: Hook event types for Forge ref updates, patch lifecycle, and CI pipeline state transitions

### Modified Capabilities

- `forge`: Forge gains commit status storage, status-aware patch resolution, and branch protection config
- `ci`: CI gains a `StatusReporter` callback invoked on pipeline state transitions
- `forge-ci-trigger`: The trigger path gains a reverse channel — CI writes status back through the same Forge KV store it reads config from

## Impact

- **crates/aspen-forge/**: New `status/` module for `CommitStatus` types and `StatusStore`. New `protection/` module for branch rules. Changes to `cob/patch.rs` for CI-aware resolution. New gossip `Announcement::PipelineStatus` variant.
- **crates/aspen-ci/**: New `StatusReporter` trait. Changes to `orchestrator/pipeline/status.rs` to call reporter on transitions. Changes to `orchestrator/pipeline/persistence.rs` to write commit statuses alongside run data.
- **crates/aspen-hooks/**: New `HookEventType` variants for forge and CI events.
- **crates/aspen-core/**: No changes — uses existing `KeyValueStore` trait.
- **KV namespace**: New prefix `forge:status:{repo_hex}:{commit_hex}:{context}` for commit statuses. New prefix `forge:protection:{repo_hex}:{ref_pattern}` for branch rules.
- **Gossip protocol**: New `Announcement::PipelineStatus` variant — additive, no breaking change to existing message handling.
- **Feature flags**: `commit-status` gated behind `forge` + `ci` features both enabled. `merge-gating` behind `forge` feature.
