Evidence-ID: no-std-aspen-core.dep-review.syn
Task-ID: 2.10
Artifact-Type: dependency-review-note
Covers: core.no-std-core-baseline, architecture.modularity.feature-bundles-are-explicit-and-bounded

Crate: syn 2.0.117
Introduced by: aspen-cluster-types@0.1.0 via serde_derive/thiserror-impl
Resolved features: clone-impls, default, derive, full, parsing, printing, proc-macro, visit-mut
Filesystem: no - syntax tree parser only.
Process/global state: no - compile-time helper only.
Thread/async-runtime: no - no runtime facilities.
Network: no - no network surface.
Decision: allow
