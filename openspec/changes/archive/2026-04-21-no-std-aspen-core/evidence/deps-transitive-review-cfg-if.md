Evidence-ID: no-std-aspen-core.dep-review.cfg-if
Task-ID: 2.10
Artifact-Type: dependency-review-note
Covers: core.no-std-core-baseline, architecture.modularity.feature-bundles-are-explicit-and-bounded

Crate: cfg-if 1.0.4
Introduced by: aspen-hlc@0.1.0 via blake3@1.8.3
Resolved features: default
Filesystem: no - compile-time cfg helper only.
Process/global state: no - no ambient state reads of its own.
Thread/async-runtime: no - no runtime or threading facilities.
Network: no - no network surface.
Decision: allow
