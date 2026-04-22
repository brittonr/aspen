Evidence-ID: no-std-aspen-core.dep-review.spin
Task-ID: 2.10
Artifact-Type: dependency-review-note
Covers: core.no-std-core-baseline, architecture.modularity.feature-bundles-are-explicit-and-bounded

Crate: spin 0.10.0
Introduced by: aspen-hlc@0.1.0 via uhlc@0.8.2
Resolved features: barrier, default, lazy, lock_api, lock_api_crate, mutex, once, rwlock, spin_mutex
Filesystem: no - synchronization primitive only.
Process/global state: no - no ambient process/global state APIs.
Thread/async-runtime: yes - synchronization crate used for no-std locking.
Network: no - no network surface.
Decision: allow
