Evidence-ID: alloc-safe-cluster-ticket.i1-implementation
Task-ID: I1
Artifact-Type: source-citation
Covers: architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.bare-cluster-ticket-dependency-stays-alloc-safe, architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.topic-and-bootstrap-roundtrip-in-alloc-safe-form, architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.nodeaddress-dependency-edge-stays-alloc-safe

## Source citations

- `crates/aspen-ticket/Cargo.toml` keeps the `aspen-cluster-types` edge alloc-safe with `default-features = false` and moves runtime opt-ins behind explicit features.
- `crates/aspen-ticket/src/lib.rs` makes the crate alloc-safe by default with `no_std`/`alloc` scaffolding and feature-gated runtime exports.
- `crates/aspen-ticket/src/v2.rs` stores unsigned ticket data as `ClusterTopicId` plus `NodeAddress`-backed bootstrap peers.
- `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/evidence/implementation-diff.txt` captures the full seam diff from `83c6480f2` to the current working tree.

## Verification cross-check

- `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/evidence/alloc-safe-validation.md`
- `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/evidence/default-vs-no-default-equivalence.md`
