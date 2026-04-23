Evidence-ID: alloc-safe-cluster-ticket.v1-unsigned-wire-break
Task-ID: V2
Artifact-Type: command-transcript
Covers: architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.topic-and-bootstrap-roundtrip-in-alloc-safe-form, architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.alloc-safe-topic-identifier-stays-fixed-width-and-lossless-at-the-iroh-boundary, architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.parse-and-validation-failures-use-cluster-ticket-errors, architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.legacy-unsigned-cluster-tickets-are-rejected-explicitly, architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.unsigned-wire-break-proof-is-reviewable

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo test -p aspen-ticket --lib test_serialize_deserialize -- --exact'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: unknown lint: `unchecked_narrowing`
  --> crates/aspen-ticket/src/lib.rs:62:13
   |
62 |     #[allow(unchecked_narrowing, reason = "port values are controlled in tests, always fit in u8 range")]
   |             ^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `numeric_units`
  --> crates/aspen-ticket/src/lib.rs:71:17
   |
71 |         #[allow(numeric_units, reason = "transport_addrs is a BTreeSet, not a numeric quantity")]
   |                 ^^^^^^^^^^^^^

warning: `aspen-ticket` (lib test) generated 2 warnings
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.35s
     Running unittests src/lib.rs (target/debug/deps/aspen_ticket-b0bdb2507e70c8c5)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 50 filtered out; finished in 0.00s


## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo test -p aspen-ticket --test errors'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.30s
     Running tests/errors.rs (target/debug/deps/errors-e4a76ce567e28bae)

running 1 test
test malformed_unsigned_ticket_returns_deserialize_error ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo test -p aspen-ticket --test topic'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.32s
     Running tests/topic.rs (target/debug/deps/topic-649a067a85fdb4e9)

running 1 test
test cluster_topic_id_rejects_wrong_length ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo test -p aspen-ticket --features iroh --test topic'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
   Compiling aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.68s
     Running tests/topic.rs (target/debug/deps/topic-915a45afa50287ed)

running 2 tests
test cluster_topic_id_rejects_wrong_length ... ok
test cluster_topic_id_roundtrips_to_iroh_topic ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo test -p aspen-ticket --test legacy'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.36s
     Running tests/legacy.rs (target/debug/deps/legacy-69a19f987bab04a1)

running 1 test
test legacy_unsigned_ticket_is_rejected ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


## Regeneration guidance

Legacy unsigned tickets now fail to decode by design. Regenerate them with the current alloc-safe schema by rebuilding the ticket through the current `AspenClusterTicket` constructors and serialization path before reusing the shared string.
