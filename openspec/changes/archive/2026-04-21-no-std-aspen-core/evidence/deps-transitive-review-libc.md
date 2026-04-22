Evidence-ID: no-std-aspen-core.dep-review.libc
Task-ID: 2.10
Artifact-Type: dependency-review-note
Covers: core.no-std-core-baseline, architecture.modularity.feature-bundles-are-explicit-and-bounded

Crate: libc 0.2.183
Introduced by: aspen-hlc@0.1.0 today; also reachable via aspen-storage-types@0.1.0/redb
Resolved features: default, std
Filesystem: yes - libc exposes filesystem/syscall bindings.
Process/global state: yes - libc exposes process-global and ambient OS APIs.
Thread/async-runtime: yes - libc exposes low-level thread/runtime primitives.
Network: yes - libc exposes socket APIs.
Decision: allow
