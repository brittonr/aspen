Evidence-ID: cluster-types-iroh-opt-in.v1-validation
Task-ID: V1
Artifact-Type: verification
Covers: architecture.modularity.acyclic-no-std-core-boundary.cluster-types-default-to-alloc-safe-builds, architecture.modularity.acyclic-no-std-core-boundary.cluster-types-verification-is-reviewable, architecture.modularity.feature-bundles-are-explicit-and-bounded.cluster-types-iroh-opt-in-is-explicit

# Cluster-types validation

## cargo check -p aspen-cluster-types --no-default-features
```
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.20s
```

## cargo check -p aspen-cluster-types --no-default-features --target wasm32-unknown-unknown
```
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.21s
```

## cargo test -p aspen-cluster-types
```
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.18s
     Running unittests src/lib.rs (target/debug/deps/aspen_cluster_types-96a21fbc26eafb4d)

running 69 tests
test tests::add_learner_request_creation ... ok
test tests::change_membership_request_empty_members ... ok
test tests::change_membership_request_creation ... ok
test tests::add_learner_request_serialization_roundtrip ... ok
test tests::change_membership_request_serialization_roundtrip ... ok
test tests::cluster_metrics_clone ... ok
test tests::cluster_metrics_creation ... ok
test tests::cluster_metrics_follower ... ok
test tests::cluster_metrics_debug ... ok
test tests::cluster_metrics_with_replication ... ok
test tests::cluster_node_clone ... ok
test tests::cluster_node_debug_format ... ok
test tests::cluster_node_equality ... ok
test tests::cluster_node_new_creates_with_legacy_address ... ok
test tests::cluster_node_new_without_raft_addr ... ok
test tests::cluster_node_serialization_roundtrip ... ok
test tests::cluster_state_clone ... ok
test tests::cluster_state_default_is_empty ... ok
test tests::cluster_state_with_nodes ... ok
test tests::control_plane_error_clone ... ok
test tests::cluster_state_serialization_roundtrip ... ok
test tests::control_plane_error_debug ... ok
test tests::control_plane_error_equality ... ok
test tests::control_plane_error_failed_display ... ok
test tests::control_plane_error_invalid_request_display ... ok
test tests::control_plane_error_not_initialized_display ... ok
test tests::control_plane_error_timeout_display ... ok
test tests::control_plane_error_unsupported_display ... ok
test tests::init_request_validate_duplicate_node_ids_fails ... ok
test tests::init_request_validate_empty_members_fails ... ok
test tests::init_request_serialization_roundtrip ... ok
test tests::init_request_validate_large_node_ids_succeeds ... ok
test tests::init_request_validate_node_id_zero_fails ... ok
test tests::init_request_validate_multiple_valid_nodes_succeeds ... ok
test tests::node_address_debug ... ok
test tests::node_address_clone ... ok
test tests::init_request_validate_node_id_zero_in_middle_fails ... ok
test tests::node_id_copy ... ok
test tests::node_id_default ... ok
test tests::node_id_from_str_negative ... ok
test tests::node_id_from_u64 ... ok
test tests::node_id_from_str_invalid ... ok
test tests::node_id_ordering ... ok
test tests::node_state_can_serve_reads ... ok
test tests::node_id_serialization_roundtrip ... ok
test tests::node_state_equality ... ok
test tests::node_state_is_leader ... ok
test tests::node_state_hash ... ok
test tests::node_id_max_value ... ok
test tests::node_id_clone ... ok
test tests::node_address_equality ... ok
test tests::snapshot_log_id_clone ... ok
test tests::node_id_display ... ok
test tests::snapshot_log_id_copy ... ok
test tests::snapshot_log_id_creation ... ok
test tests::snapshot_log_id_debug ... ok
test tests::snapshot_log_id_equality ... ok
test tests::node_id_equality ... ok
test tests::node_id_from_str ... ok
test tests::node_id_hash ... ok
test tests::node_id_new ... ok
test tests::node_id_into_u64 ... ok
test tests::node_address_from_parts_preserves_id_and_display ... ok
test tests::node_state_as_u8 ... ok
test tests::init_request_validate_single_valid_node_succeeds ... ok
test tests::node_state_debug ... ok
test tests::node_state_copy ... ok
test tests::node_state_clone ... ok
test tests::node_state_is_healthy ... ok

test result: ok. 69 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests aspen_cluster_types

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

## cargo test -p aspen-cluster-types --features iroh
```
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.19s
     Running unittests src/lib.rs (target/debug/deps/aspen_cluster_types-76f545968d55453f)

running 75 tests
test tests::change_membership_request_creation ... ok
test tests::add_learner_request_creation ... ok
test tests::change_membership_request_empty_members ... ok
test tests::cluster_metrics_clone ... ok
test tests::change_membership_request_serialization_roundtrip ... ok
test tests::add_learner_request_serialization_roundtrip ... ok
test tests::cluster_metrics_creation ... ok
test tests::cluster_metrics_debug ... ok
test tests::cluster_metrics_follower ... ok
test tests::cluster_metrics_with_replication ... ok
test tests::cluster_node_clone ... ok
test tests::cluster_node_equality ... ok
test tests::cluster_node_debug_format ... ok
test tests::cluster_node_iroh_addr_returns_none_when_not_set ... ok
test tests::cluster_node_new_creates_with_legacy_address ... ok
test tests::cluster_node_new_without_raft_addr ... ok
test tests::cluster_node_serialization_roundtrip ... ok
test tests::cluster_state_clone ... ok
test tests::cluster_state_default_is_empty ... ok
test tests::cluster_state_serialization_roundtrip ... ok
test tests::cluster_state_with_nodes ... ok
test tests::control_plane_error_clone ... ok
test tests::control_plane_error_debug ... ok
test tests::control_plane_error_equality ... ok
test tests::control_plane_error_failed_display ... ok
test tests::control_plane_error_invalid_request_display ... ok
test tests::cluster_node_with_node_addr ... ok
test tests::control_plane_error_not_initialized_display ... ok
test tests::control_plane_error_timeout_display ... ok
test tests::cluster_node_with_iroh_addr ... ok
test tests::control_plane_error_unsupported_display ... ok
test tests::init_request_validate_duplicate_node_ids_fails ... ok
test tests::init_request_serialization_roundtrip ... ok
test tests::init_request_validate_empty_members_fails ... ok
test tests::init_request_validate_large_node_ids_succeeds ... ok
test tests::init_request_validate_multiple_valid_nodes_succeeds ... ok
test tests::init_request_validate_node_id_zero_fails ... ok
test tests::init_request_validate_node_id_zero_in_middle_fails ... ok
test tests::init_request_validate_single_valid_node_succeeds ... ok
test tests::node_address_clone ... ok
test tests::node_address_debug ... ok
test tests::node_address_equality ... ok
test tests::node_address_from_parts_preserves_id_and_display ... ok
test tests::node_address_try_into_iroh_rejects_invalid_endpoint_id ... ok
test tests::node_id_clone ... ok
test tests::node_id_copy ... ok
test tests::node_id_default ... ok
test tests::node_id_display ... ok
test tests::node_id_equality ... ok
test tests::node_id_from_str ... ok
test tests::node_id_from_str_invalid ... ok
test tests::node_id_from_str_negative ... ok
test tests::node_address_new_round_trips_endpoint_addr ... ok
test tests::node_id_from_u64 ... ok
test tests::node_id_into_u64 ... ok
test tests::node_id_hash ... ok
test tests::node_address_try_into_iroh_rejects_invalid_relay_url ... ok
test tests::node_id_max_value ... ok
test tests::node_id_new ... ok
test tests::node_id_ordering ... ok
test tests::node_state_as_u8 ... ok
test tests::node_id_serialization_roundtrip ... ok
test tests::node_state_can_serve_reads ... ok
test tests::node_state_hash ... ok
test tests::snapshot_log_id_creation ... ok
test tests::node_state_clone ... ok
test tests::node_state_debug ... ok
test tests::node_state_is_leader ... ok
test tests::snapshot_log_id_copy ... ok
test tests::snapshot_log_id_clone ... ok
test tests::node_state_equality ... ok
test tests::snapshot_log_id_equality ... ok
test tests::node_state_copy ... ok
test tests::node_state_is_healthy ... ok
test tests::snapshot_log_id_debug ... ok

test result: ok. 75 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests aspen_cluster_types

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

## cargo tree -p aspen-cluster-types -e normal --depth 2
```
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
├── serde v1.0.228
│   ├── serde_core v1.0.228
│   └── serde_derive v1.0.228 (proc-macro)
└── thiserror v2.0.18
    └── thiserror-impl v2.0.18 (proc-macro)
```

## cargo tree -p aspen-cluster-types --features iroh -e normal --depth 2
```
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
├── iroh-base v0.97.0
│   ├── curve25519-dalek v5.0.0-pre.1
│   ├── data-encoding v2.10.0
│   ├── derive_more v2.1.1
│   ├── digest v0.11.0-rc.10
│   ├── ed25519-dalek v3.0.0-pre.1
│   ├── n0-error v0.1.3
│   ├── rand_core v0.9.5
│   ├── serde v1.0.228
│   ├── sha2 v0.11.0-rc.2
│   ├── url v2.5.8
│   ├── zeroize v1.8.2
│   └── zeroize_derive v1.4.3 (proc-macro)
├── serde v1.0.228 (*)
└── thiserror v2.0.18
    └── thiserror-impl v2.0.18 (proc-macro)
```

## cargo tree -p aspen-traits -e normal --depth 2
```
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
│   ├── serde v1.0.228
│   └── thiserror v2.0.18
├── aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types)
│   ├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
│   ├── serde v1.0.228 (*)
│   └── thiserror v2.0.18 (*)
└── async-trait v0.1.89 (proc-macro)
    ├── proc-macro2 v1.0.106
    ├── quote v1.0.45
    └── syn v2.0.117
```

