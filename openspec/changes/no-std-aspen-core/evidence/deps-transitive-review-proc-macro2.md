Evidence-ID: no-std-aspen-core.dep-review.proc-macro2
Task-ID: 2.10
Artifact-Type: dependency-review-note
Covers: core.no-std-core-baseline, architecture.modularity.feature-bundles-are-explicit-and-bounded

Crate: proc-macro2 1.0.106
Introduced by: aspen-cluster-types@0.1.0 via serde_derive/thiserror-impl
Resolved features: default, proc-macro
Filesystem: no - token-stream support only.
Process/global state: no - proc-macro infrastructure only.
Thread/async-runtime: no - compile-time helper only.
Network: no - no network surface.
Decision: allow
