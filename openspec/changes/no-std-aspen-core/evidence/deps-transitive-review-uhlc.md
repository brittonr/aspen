Evidence-ID: no-std-aspen-core.dep-review.uhlc
Task-ID: 2.10
Artifact-Type: dependency-review-note
Covers: core.no-std-core-baseline, architecture.modularity.feature-bundles-are-explicit-and-bounded

Crate: uhlc 0.8.2
Introduced by: aspen-hlc@0.1.0
Resolved features: (none)
Filesystem: no - logical-clock library only.
Process/global state: no - no ambient process/global reads in the wrapped API surface.
Thread/async-runtime: yes - pulls in spin-based synchronization and currently drags randomness transitively.
Network: no - no network surface.
Decision: allow
