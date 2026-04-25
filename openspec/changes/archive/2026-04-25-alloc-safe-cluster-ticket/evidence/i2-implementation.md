Evidence-ID: alloc-safe-cluster-ticket.i2-implementation
Task-ID: I2
Artifact-Type: source-citation
Covers: architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.alloc-safe-topic-identifier-stays-fixed-width-and-lossless-at-the-iroh-boundary, architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.parse-and-validation-failures-use-cluster-ticket-errors, architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.legacy-unsigned-cluster-tickets-are-rejected-explicitly, architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.unsigned-wire-break-proof-is-reviewable

## Source citations

- `crates/aspen-ticket/src/v2.rs` defines `ClusterTicketError`, fixed-width `ClusterTopicId`, and alloc-safe validation helpers.
- `crates/aspen-ticket/src/parse.rs` returns crate-local deserialize errors on malformed unsigned input.
- `crates/aspen-ticket/tests/errors.rs` proves malformed unsigned ticket strings fail through `ClusterTicketError::Deserialize`.
- `crates/aspen-ticket/tests/topic.rs` proves malformed topic widths fail through `ClusterTicketError::InvalidTopicId` and that the optional iroh topic conversion stays lossless.
- `crates/aspen-ticket/tests/legacy.rs` proves legacy unsigned payloads are rejected and require regeneration.

## Verification cross-check

- `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/evidence/unsigned-wire-break.md`
- `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/evidence/implementation-diff.txt`
