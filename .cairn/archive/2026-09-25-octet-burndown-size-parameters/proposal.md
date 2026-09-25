# Proposal: Octet burn-down, explicit input structs for long parameter lists

## Why

C3 is the size-shape set of Octet families. Its first slice is `too_many_parameters`: 143 workspace findings at 72
functions that take six or more parameters. The base is `cec959753` on `integration/stack-20260925` (`70a9f0b53`
plus C4c2 and the README `default-run` fix), with 3207 workspace findings and 1363 lib findings. The family must reach
zero by repair. The size-shape slices that follow are `borrowed_argument_types`, `function_length`, and
`excessive_file_length`.

## What Changes

- Every flagged function keeps its name. Its trailing related parameters move into one explicit `*Input` struct (or a
  named group such as `ReplicaAdapterSet`, `AdapterDelivery`, `EventHeader`, `SignerIdentity`, or `AdmissionSet`).
- The function destructures the struct at the top of its body, so the body logic does not change. Call sites build
  the struct with named fields, so arguments of the same type are no longer bound by position.
- Parameter groups that repeat across functions share one struct: `DependencyEdgeInput`, `RunArtifacts`,
  `ActionPorts`, `ReplicaAdapterSet`, `AdapterDelivery`, `EventHeader`, and `FailureEvidence`.
- One parameter that was never used (`duplicate_or_conflict_decision`'s `_input`) is removed.
- Incidental fixes keep every other family level:
  - `bounded_content_status` and `redb_index_content_status` move unchanged from `content_store_adapter/local.rs` to
    `content_store_adapter/integration.rs`, which keeps `local.rs` under the file-length limit.
  - The observability tests gain a `shell_state` helper.

## Impact

- **Files**: 75 files in `src/` and 1 in `tests/`.
- **Public API**: public functions change their parameter shape, and none is renamed. They include
  `simulate_actor_sequence`, `claim_capability_request`, `ReplicaPortBundle::new`,
  `assemble_scoped_concrete_replica_ports`, `validate_concrete_replica_port_identity`, `canonical_key_handle`,
  `IrohEd25519FileAdapter::verify`, `execute_snapshot_export`, `execute_event_export`, `runtime_counter_sample`,
  `canonical_simulation_differential`, `canonical_named_event`, `live_loopback_frame`,
  `execute_local_stream_put`/`_get`, `execute_live_iroh_stream_get`, `execute_simulated_stream`,
  `bounded_content_status`, `SystemExtensionHost::from_recovered_state`, `NativeSystemExtensionService::install`,
  `NativeSystemExtensionService::from_recovered`, and `LocalWorldHeadSigningAdapter::new`.
- **Testing**:
  - Pinned Octet root and lib runs.
  - fmt, and clippy `-D warnings`.
  - Full `cargo test --workspace`, including `fabricboundarycompat` and the `fabric_execution::` tests.
  - Harness and fabric-simulation byte identity against the base binary.

## Out of Scope

- No renames. The C2 naming-policy decision stays with the owner.
- `borrowed_argument_types`, `function_length`, and `excessive_file_length` are handled in later slices. This slice
  only keeps them from growing.
