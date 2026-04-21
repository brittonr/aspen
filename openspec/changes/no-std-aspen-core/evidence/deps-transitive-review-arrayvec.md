Evidence-ID: no-std-aspen-core.dep-review.arrayvec
Task-ID: 2.10
Artifact-Type: dependency-review-note
Covers: core.no-std-core-baseline, architecture.modularity.feature-bundles-are-explicit-and-bounded

Crate: arrayvec 0.7.6
Introduced by: aspen-hlc@0.1.0 via blake3@1.8.3
Resolved features: (none)
Filesystem: no - inline vec storage only.
Process/global state: no - no ambient process state APIs.
Thread/async-runtime: no - no runtime or threading facilities.
Network: no - no network surface.
Decision: allow
