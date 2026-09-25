# Change-local acceptance

a[pin-fabric-boundary-compatibility-fixtures.premigration] The fixtures are generated at the pre-migration commit `3de348149^` from the shared explicit inputs, and the generator, commit, and input and fixture hashes are recorded in `evidence/`.
a[pin-fabric-boundary-compatibility-fixtures.equality] For equal explicit inputs, the membership profile and view, the assignment transition, the time profile, the transport profile and transition, and the durable profile and transition produce byte-equal canonical values and equal refs against the pinned fixtures.
a[pin-fabric-boundary-compatibility-fixtures.sensitivity] A one-field input mutation changes each covered projection's ref, and a tampered or truncated fixture fails strict canonical decode under its pinned ref.
a[pin-fabric-boundary-compatibility-fixtures.traceability] Both accepted ids carry evidence markers. `inherited-tracey-debt` reports unexpected_missing dropping from 3 to 1 with no new dangling reference, and fmt, clippy with `-D warnings`, and `cargo test --workspace` pass.
a[pin-fabric-boundary-compatibility-fixtures.octet] Pinned `cargo octet check` (root, and `-p molten --lib`) shows a zero per-lint delta against the base `d825dc14b`. The new tests use no `expect`, `unwrap`, or lint allows.
