Evidence-ID: aspen-core-no-std.dep-review.blake3
Artifact-Type: dependency-review-note
Covers: core.no-std-core-baseline, architecture.modularity.feature-bundles-are-explicit-and-bounded

Crate: blake3 1.8.3
Introduced by: aspen-hlc@0.1.0
Resolved features: (none)
Filesystem: no - hashing implementation only.
Process/global state: no - deterministic hash computation only.
Thread/async-runtime: no - no async runtime dependency in this slice.
Network: no - no network surface.
Decision: allow
Notes: Aspen pins 1.8.3 while the current snix castore revision still expects the digest 0.10 trait family; 1.8.5 moves its digest feature to digest 0.11 and breaks that shared build graph.
