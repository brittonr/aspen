Evidence-ID: aspen-core-no-std.dep-review.cobs
Artifact-Type: dependency-review-note
Covers: core.no-std-core-baseline, architecture.modularity.feature-bundles-are-explicit-and-bounded

Crate: cobs 0.3.0
Introduced by: aspen-codec@0.1.0 via postcard@1.1.3
Resolved features: default
Filesystem: no - Consistent Overhead Byte Stuffing codec only.
Process/global state: no - deterministic byte encoding/decoding only.
Thread/async-runtime: no - no async runtime dependency.
Network: no - no network surface.
Decision: allow
