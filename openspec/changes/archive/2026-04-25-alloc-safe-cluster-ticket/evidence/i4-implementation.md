Evidence-ID: alloc-safe-cluster-ticket.i4-implementation
Task-ID: I4
Artifact-Type: source-citation
Covers: architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.iroh-conversion-happens-at-the-shell-boundary

## Source citations

- `Cargo.toml` keeps the workspace `aspen-ticket` stanza alloc-safe with `default-features = false`.
- `crates/aspen-client/Cargo.toml`, `crates/aspen-cluster-handler/Cargo.toml`, and `crates/aspen-cluster/Cargo.toml` opt into runtime features only where needed.
- `crates/aspen-client/src/client.rs` converts alloc-safe bootstrap peers to runtime endpoint addresses at `peer.to_endpoint_addr()`.
- `crates/aspen-cluster-handler/src/handler/tickets.rs` constructs runtime tickets from `TopicId`, `EndpointId`, and `EndpointAddr` helpers.
- `crates/aspen-cluster/src/ticket.rs`, `crates/aspen-cluster/src/bootstrap/node/discovery_init.rs`, and `crates/aspen-cluster/src/bootstrap/node/sharding_init.rs` keep runtime topic conversion at the cluster shell boundary.

## Verification cross-check

- `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/evidence/direct-consumer-audit.md`
- `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/evidence/workspace-dependency-proof.txt`
- `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/evidence/implementation-diff.txt`
