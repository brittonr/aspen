Evidence-ID: alloc-safe-cluster-ticket.v1-unsigned-wire-break
Task-ID: V2
Artifact-Type: command-transcript
Covers: architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.topic-and-bootstrap-roundtrip-in-alloc-safe-form, architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.alloc-safe-topic-identifier-stays-fixed-width-and-lossless-at-the-iroh-boundary, architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.parse-and-validation-failures-use-cluster-ticket-errors, architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.legacy-unsigned-cluster-tickets-are-rejected-explicitly, architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.unsigned-wire-break-proof-is-reviewable

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo test -p aspen-ticket test_serialize_deserialize -- --exact'`


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 50 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo test -p aspen-ticket test_deserialize_invalid -- --exact'`


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 50 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo test -p aspen-ticket --test topic'`


running 1 test
test cluster_topic_id_rejects_wrong_length ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo test -p aspen-ticket --features iroh --test topic'`


running 2 tests
test cluster_topic_id_rejects_wrong_length ... ok
test cluster_topic_id_roundtrips_to_iroh_topic ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo test -p aspen-ticket --test legacy'`


running 1 test
test legacy_unsigned_ticket_is_rejected ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


## Regeneration guidance

Legacy unsigned tickets now fail to decode by design. Regenerate them with the current alloc-safe schema by rebuilding the ticket through the current `AspenClusterTicket` constructors/serialization path before reusing the shared string.
