Evidence-ID: no-std-aspen-core.dep-review.quote
Task-ID: 2.10
Artifact-Type: dependency-review-note
Covers: core.no-std-core-baseline, architecture.modularity.feature-bundles-are-explicit-and-bounded

Crate: quote 1.0.45
Introduced by: aspen-cluster-types@0.1.0 via serde_derive/thiserror-impl
Resolved features: default, proc-macro
Filesystem: no - token emission helper only.
Process/global state: no - compile-time helper only.
Thread/async-runtime: no - no runtime facilities.
Network: no - no network surface.
Decision: allow
