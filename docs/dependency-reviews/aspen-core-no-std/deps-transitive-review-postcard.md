Evidence-ID: aspen-core-no-std.dep-review.postcard
Artifact-Type: dependency-review-note
Covers: core.no-std-core-baseline, architecture.modularity.feature-bundles-are-explicit-and-bounded

Crate: postcard 1.1.3
Introduced by: aspen-codec@0.1.0
Resolved features: alloc
Filesystem: no - serialization codec only.
Process/global state: no - deterministic serde-compatible byte encoding/decoding only.
Thread/async-runtime: no - no async runtime dependency.
Network: no - no network surface.
Decision: allow
