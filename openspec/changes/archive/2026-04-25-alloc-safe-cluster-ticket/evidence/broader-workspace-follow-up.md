Evidence-ID: alloc-safe-cluster-ticket.v6-broader-workspace-follow-up
Task-ID: V6
Artifact-Type: command-transcript
Covers: architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.cluster-ticket-seam-proof-is-reviewable

## Broader workspace rails

- `cargo check -p aspen-cli` → pass
- `cargo check -p aspen-fuse` → pass
- `cargo check -p aspen --no-default-features --features node-runtime` → pass after fixing the cluster bootstrap `ClusterTopicId` → `TopicId` runtime conversion sites.

## Follow-up fix

- `crates/aspen-cluster/src/bootstrap/node/discovery_init.rs` now uses `ticket.topic_id.to_topic_id()`.
- `crates/aspen-cluster/src/bootstrap/node/sharding_init.rs` now uses `ticket.topic_id.to_topic_id()`.

## Durable transcript

See `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/evidence/final-validation.md` for the full command output and fallout note.
