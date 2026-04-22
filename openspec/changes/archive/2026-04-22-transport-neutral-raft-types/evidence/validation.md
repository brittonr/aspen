Evidence-ID: transport-neutral-raft-types.validation
Task-ID: V1
Artifact-Type: command-transcript
Covers: transport.transport-neutral-raft-membership-metadata, transport.transport-neutral-raft-membership-metadata.membership-metadata-stays-transport-neutral-at-rest, transport.transport-neutral-raft-membership-metadata.runtime-conversion-failure-is-surfaced-explicitly, transport.transport-neutral-raft-membership-metadata.default-member-metadata-stays-parseable-for-runtime-helpers

# Targeted Validation

## cargo test -p aspen-raft-types

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.25s
     Running unittests src/lib.rs (target/debug/deps/aspen_raft_types-df5a778c6a81950e)

running 56 tests
test constants::tests::test_snapshot_threshold_above_safety_floor ... ok
test constants::tests::test_timeout_ordering ... ok
test member::tests::test_raft_member_info_clone ... ok
test member::tests::test_raft_member_info_default ... ok
test member::tests::test_raft_member_info_display ... ok
test member::tests::test_raft_member_info_equality ... ok
test member::tests::test_raft_member_info_new ... ok
test member::tests::test_raft_member_info_with_relay ... ok
test member::tests::test_raft_member_info_serde_roundtrip ... ok
test network::tests::test_connection_health_copy ... ok
test network::tests::test_connection_health_debug ... ok
test network::tests::test_connection_health_degraded ... ok
test network::tests::test_connection_health_eq_different_variants ... ok
test network::tests::test_connection_health_eq_same_degraded_different_counts ... ok
test network::tests::test_connection_health_failed ... ok
test network::tests::test_connection_health_healthy ... ok
test network::tests::test_connection_status_copy ... ok
test network::tests::test_connection_health_serde_roundtrip ... ok
test network::tests::test_connection_status_debug ... ok
test network::tests::test_connection_status_equality ... ok
test network::tests::test_connection_status_serde_roundtrip ... ok
test network::tests::test_drift_severity_copy ... ok
test network::tests::test_drift_severity_debug ... ok
test network::tests::test_drift_severity_equality ... ok
test network::tests::test_drift_severity_serde_roundtrip ... ok
test network::tests::test_failure_type_copy ... ok
test network::tests::test_failure_type_debug ... ok
test network::tests::test_failure_type_equality ... ok
test network::tests::test_failure_type_serde_roundtrip ... ok
test network::tests::test_stream_priority_copy ... ok
test network::tests::test_stream_priority_critical_higher_than_bulk ... ok
test network::tests::test_stream_priority_equality ... ok
test network::tests::test_stream_priority_serde_roundtrip ... ok
test network::tests::test_stream_priority_to_i32 ... ok
test request::tests::test_app_request_delete_display ... ok
test request::tests::test_app_request_delete_multi_empty ... ok
test request::tests::test_app_request_delete_multi_many ... ok
test request::tests::test_app_request_serde_delete ... ok
test request::tests::test_app_request_serde_delete_multi ... ok
test request::tests::test_app_request_serde_set ... ok
test request::tests::test_app_request_serde_set_multi ... ok
test request::tests::test_app_request_set_display ... ok
test request::tests::test_app_request_set_multi_empty ... ok
test request::tests::test_app_request_set_multi_many ... ok
test request::tests::test_app_request_set_multi_one ... ok
test request::tests::test_app_request_trust_initialize_display ... ok
test request::tests::test_app_request_serde_trust_initialize ... ok
test request::tests::test_app_request_trust_reconfiguration_display ... ok
test request::tests::test_app_response_cas_failed ... ok
test request::tests::test_app_request_serde_trust_reconfiguration ... ok
test request::tests::test_app_response_cas_succeeded ... ok
test request::tests::test_app_response_deleted_false ... ok
test request::tests::test_app_response_deleted_true ... ok
test request::tests::test_app_response_cas_serde_roundtrip ... ok
test request::tests::test_app_response_with_value ... ok
test request::tests::test_app_response_serde_roundtrip ... ok

test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests aspen_raft_types

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


## cargo check -p aspen-raft

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.26s

## cargo check -p aspen-cluster

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.31s

## cargo check -p aspen-raft --features trust

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.29s

## cargo tree -p aspen-raft-types -e normal --depth 1

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-raft-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-types)
├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
├── aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core)
├── aspen-trust v0.1.0 (/home/brittonr/git/aspen/crates/aspen-trust)
├── irpc v0.13.0
├── openraft v0.10.0 (/home/brittonr/git/aspen/openraft/openraft)
├── serde v1.0.228
└── tracing v0.1.44

## cargo test -p aspen-raft test_raft_member_info_default_endpoint_id_is_parseable --lib

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.33s
     Running unittests src/lib.rs (target/debug/deps/aspen_raft-20690b57a5f7d214)

running 1 test
test network::tests::test_raft_member_info_default_endpoint_id_is_parseable ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 710 filtered out; finished in 0.00s


## cargo test -p aspen-raft test_member_endpoint_addr_rejects_invalid_node_address --lib

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.32s
     Running unittests src/lib.rs (target/debug/deps/aspen_raft-20690b57a5f7d214)

running 1 test
test types::tests::test_member_endpoint_addr_rejects_invalid_node_address ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 710 filtered out; finished in 0.00s


## cargo test -p aspen-raft test_raft_member_info_construction --lib

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.31s
     Running unittests src/lib.rs (target/debug/deps/aspen_raft-20690b57a5f7d214)

running 1 test
test network::tests::test_raft_member_info_construction ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 710 filtered out; finished in 0.01s


## cargo test -p aspen-raft --features trust test_collect_old_shares_accepts_valid_remote_share_from_stored_address --lib

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.29s
     Running unittests src/lib.rs (target/debug/deps/aspen_raft-98bae123d4e0bdb2)

running 1 test
test node::trust::tests::test_collect_old_shares_accepts_valid_remote_share_from_stored_address ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 729 filtered out; finished in 0.00s


## cargo test -p aspen-cluster --features relay-server test_extract_endpoint_ids_skips_invalid_member_endpoint_id --lib

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
   Compiling aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 5.68s
     Running unittests src/lib.rs (target/debug/deps/aspen_cluster-f54d0e3631c0c099)

running 1 test
test relay_server::tests::test_extract_endpoint_ids_skips_invalid_member_endpoint_id ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 218 filtered out; finished in 0.00s
