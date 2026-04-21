Evidence-ID: no-std-aspen-core.dep-review.cpufeatures
Task-ID: 2.10
Artifact-Type: dependency-review-note
Covers: core.no-std-core-baseline, architecture.modularity.feature-bundles-are-explicit-and-bounded

Crate: cpufeatures 0.2.17
Introduced by: aspen-hlc@0.1.0 via blake3@1.8.3
Resolved features: default
Filesystem: no - CPU capability detection only.
Process/global state: no - hardware probing, not ambient process-global mutable state.
Thread/async-runtime: no - no async runtime dependency.
Network: no - no network surface.
Decision: allow
