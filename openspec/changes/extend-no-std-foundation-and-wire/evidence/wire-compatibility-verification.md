Evidence-ID: extend-no-std-foundation-and-wire.v4
Task-ID: V4
Artifact-Type: verification
Covers: architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.client-api-postcard-tests-use-alloc-safe-serializers, architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.wire-compatibility-survives-alloc-safe-refactor, architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.wire-compatibility-verification-is-reviewable

# Wire compatibility verification

## Harness implementation

Changed files for this task:

- `crates/aspen-client-api/Cargo.toml`
- `crates/aspen-client-api/tests/client_rpc_postcard_baseline.rs`
- `openspec/changes/extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json`

`crates/aspen-client-api/tests/client_rpc_postcard_baseline.rs` proves the wire-compatibility harness now:

- uses alloc-safe postcard serializers (`postcard::to_allocvec` and `postcard::from_bytes`) for every generated sample
- parses the live Rust source with `syn` to enumerate the current `ClientRpcRequest` and `ClientRpcResponse` variant sets instead of relying on hand-maintained lists
- derives canonical sample payloads from fixed rules only: stable path-derived strings, fixed integers, fixed bytes, deterministic single-entry collections, and recursive type expansion with a fixed depth bound
- writes and compares a deterministic baseline artifact at `openspec/changes/extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json`
- fails if the saved baseline keys no longer match the live request/response variant sets before comparing the rendered artifact byte-for-byte

## Baseline generation command

This command generated the committed baseline artifact:

```text
RUSTC_WRAPPER= ASPEN_UPDATE_CLIENT_RPC_POSTCARD_BASELINE=1 cargo test -p aspen-client-api client_rpc_postcard_baseline -- --nocapture
```

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.21s
     Running unittests src/lib.rs (target/debug/deps/aspen_client_api-619f088958270706)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 48 filtered out; finished in 0.00s

     Running tests/client_rpc_postcard_baseline.rs (target/debug/deps/client_rpc_postcard_baseline-0ba744cc521a0349)

running 1 test
updated postcard baseline artifact at /home/brittonr/git/aspen/crates/aspen-client-api/../../openspec/changes/extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json
baseline file: /home/brittonr/git/aspen/crates/aspen-client-api/../../openspec/changes/extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json
request variants: 346
response variants: 274
test client_rpc_postcard_baseline ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s

     Running tests/serialization_snapshots.rs (target/debug/deps/serialization_snapshots-6091b79e89c6f374)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s

     Running tests/snapshot_redaction.rs (target/debug/deps/snapshot_redaction-f60b9a3c8720657b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests/wire_format_golden.rs (target/debug/deps/wire_format_golden-43bed85a96a0f964)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s
```

## `cargo test -p aspen-client-api`

```text
RUSTC_WRAPPER= cargo test -p aspen-client-api
```

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.20s
     Running unittests src/lib.rs (target/debug/deps/aspen_client_api-619f088958270706)

running 48 tests
test messages::to_operation::coordination_ops::tests::test_client_rpc_lockset_requests_use_canonical_operation_member ... ok
test messages::coordination::tests::test_lockset_requests_use_canonical_operation_member ... ok
test messages::coordination::tests::test_lockset_token_requests_use_canonical_operation_member ... ok
test messages::to_operation::coordination_ops::tests::test_client_rpc_lockset_token_requests_use_canonical_operation_member ... ok
test test_authenticated_request_with_proxy_hops ... ok
test test_capability_unavailable_response ... ok
test test_blob_operations_postcard_roundtrip ... ok
test test_batch_operations_postcard_roundtrip ... ok
test test_error_response_creation_and_fields ... ok
test test_coordination_operations_postcard_roundtrip ... ok
test test_lease_operations_postcard_roundtrip ... ok
test test_to_operation_cluster_admin ... ok
test test_required_app_returns_correct_domain ... ok
test test_to_operation_kv_operations ... ok
test test_to_operation_none_for_public_operations ... ok
test test_variant_name_core_requests ... ok
test test_watch_operations_postcard_roundtrip ... ok
test tests::test_all_enum_layout_features_are_default ... ok
test tests::test_authenticated_request_from ... ok
test tests::test_authenticated_request_unauthenticated ... ok
test tests::test_client_alpn_is_correct ... ok
test tests::test_constants_are_bounded ... ok
test tests::test_deploy_required_app ... ok
test tests::test_deploy_request_discriminant_golden_table ... ok
test tests::test_deploy_variant_names ... ok
test tests::test_deploy_to_operation ... ok
test tests::test_deploy_response_discriminant_golden_table ... ok
test tests::test_error_response_discriminant_is_stable ... ok
test tests::test_feature_gated_variants_are_grouped_by_feature ... ok
test tests::test_feature_gated_response_variants_postcard_roundtrip ... ok
test tests::test_get_health_domain ... ok
test tests::test_first_and_last_response_variants_postcard_stable ... ok
test tests::test_federation_git_clone_postcard_roundtrip ... ok
test tests::test_get_health_variant_name ... ok
test tests::test_get_health_roundtrip_json ... ok
test tests::test_git_bridge_probe_objects_variant_name ... ok
test tests::test_kv_read_variant_name ... ok
test tests::test_git_bridge_probe_objects_request_roundtrip ... ok
test tests::test_git_bridge_probe_objects_response_roundtrip ... ok
test tests::test_kv_write_variant_name ... ok
test tests::test_plugin_reload_result_before_feature_gated_variants ... ok
test tests::test_request_discriminant_golden_table ... ok
test tests::test_request_metadata_lookup_matches_sample_requests ... ok
test tests::test_response_discriminant_golden_table ... ok
test tests::test_response_leader_roundtrip ... ok
test tests::test_response_pong_roundtrip ... ok
test tests::test_response_error_roundtrip ... ok
test tests::test_request_required_app_metadata_is_unique_and_known ... ok

test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/client_rpc_postcard_baseline.rs (target/debug/deps/client_rpc_postcard_baseline-0ba744cc521a0349)

running 1 test
test client_rpc_postcard_baseline ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s

     Running tests/serialization_snapshots.rs (target/debug/deps/serialization_snapshots-6091b79e89c6f374)

running 12 tests
test snapshot_delete_key_request ... ok
test snapshot_read_key_request ... ok
test snapshot_get_health_request ... ok
test snapshot_delete_result_response ... ok
test snapshot_error_response ... ok
test snapshot_read_result_response_found ... ok
test snapshot_write_key_request ... ok
test snapshot_read_result_response_not_found ... ok
test snapshot_scan_keys_request ... ok
test snapshot_scan_result_response ... ok
test snapshot_write_result_response ... ok
test snapshot_get_raft_metrics_request ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running tests/snapshot_redaction.rs (target/debug/deps/snapshot_redaction-f60b9a3c8720657b)

running 3 tests
test snapshot_error_response_with_uuid ... ok
test snapshot_error_response_with_dynamic_node_id ... ok
test snapshot_write_result_with_no_dynamic_fields ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running tests/wire_format_golden.rs (target/debug/deps/wire_format_golden-43bed85a96a0f964)

running 10 tests
test test_get_health_request_discriminant_is_0 ... ok
test test_error_response_discriminant_is_14 ... ok
test test_pong_response_discriminant_is_12 ... ok
test test_golden_files_are_sequential ... ok
test test_request_feature_gated_after_non_gated_sections ... ok
test test_request_variant_count ... ok
test test_response_variant_count ... ok
test test_response_feature_gated_ordering ... ok
test test_response_discriminants_golden ... ok
test test_request_discriminants_golden ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests aspen_client_api

running 1 test
test crates/aspen-client-api/src/lib.rs - (line 27) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

all doctests ran in 0.38s; merged doctests compilation took 0.38s
```

## `cargo test -p aspen-client-api client_rpc_postcard_baseline -- --nocapture`

```text
RUSTC_WRAPPER= cargo test -p aspen-client-api client_rpc_postcard_baseline -- --nocapture
```

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.22s
     Running unittests src/lib.rs (target/debug/deps/aspen_client_api-619f088958270706)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 48 filtered out; finished in 0.00s

     Running tests/client_rpc_postcard_baseline.rs (target/debug/deps/client_rpc_postcard_baseline-0ba744cc521a0349)

running 1 test
baseline file: /home/brittonr/git/aspen/crates/aspen-client-api/../../openspec/changes/extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json
request variants: 346
response variants: 274
test client_rpc_postcard_baseline ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s

     Running tests/serialization_snapshots.rs (target/debug/deps/serialization_snapshots-6091b79e89c6f374)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s

     Running tests/snapshot_redaction.rs (target/debug/deps/snapshot_redaction-f60b9a3c8720657b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests/wire_format_golden.rs (target/debug/deps/wire_format_golden-43bed85a96a0f964)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s
```

## Artifact checks

- `openspec/changes/extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json` now exists and contains one postcard hex encoding per live request/response variant.
- The baseline compare test printed `request variants: 346` and `response variants: 274`, proving completeness against the live source-enumerated variant sets before byte-for-byte comparison.
- The same compare test passed without update mode, proving the current wire behavior matches the saved baseline artifact exactly.
- Representative runtime consumer compile evidence remains indexed in `openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-dependency-verification.md`, specifically the saved sections for `cargo check -p aspen-cluster`, `cargo check -p aspen-client`, `cargo check -p aspen-cli`, `cargo check -p aspen-rpc-handlers`, and `cargo check -p aspen --no-default-features --features node-runtime`.
