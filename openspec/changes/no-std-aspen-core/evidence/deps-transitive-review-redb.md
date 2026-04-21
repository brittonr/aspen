Evidence-ID: no-std-aspen-core.dep-review.redb
Task-ID: 2.10
Artifact-Type: dependency-review-note
Covers: core.no-std-core-baseline, architecture.modularity.feature-bundles-are-explicit-and-bounded

Crate: redb 2.6.3
Introduced by: aspen-storage-types@0.1.0
Resolved features: default
Filesystem: yes - embedded database crate with filesystem-backed storage.
Process/global state: no - no mandatory process-global state in this type-only usage.
Thread/async-runtime: no - no async runtime dependency required for the static table definition used here.
Network: no - no network surface.
Decision: allow
