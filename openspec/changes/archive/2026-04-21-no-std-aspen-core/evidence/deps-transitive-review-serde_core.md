Evidence-ID: no-std-aspen-core.dep-review.serde_core
Task-ID: 2.10
Artifact-Type: dependency-review-note
Covers: core.no-std-core-baseline, architecture.modularity.feature-bundles-are-explicit-and-bounded

Crate: serde_core 1.0.228
Introduced by: aspen-cluster-types@0.1.0 via serde@1.0.228
Resolved features: alloc, result, std
Filesystem: no - serialization core only.
Process/global state: no - no ambient process state APIs.
Thread/async-runtime: no - no runtime facilities.
Network: no - no network surface.
Decision: allow
