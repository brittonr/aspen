## Phase 1: Core envelope spine

- [ ] [serial] r[molten.runtime_spine.envelope] Define `Envelope`, `ActorId`, `ContentRef`, `Capability`, and `EvidenceRef` core types with Serde DTO boundaries.
- [ ] [serial] r[molten.runtime_spine.canonical_preserves] Add Preserves conversion and Blake3 canonical hash tests for envelope fixtures.
- [ ] [serial] r[molten.runtime_spine.preserves_comms] Require canonical Preserves representations for actor, dataspace, choreography, consensus, transport, execution, storage, policy, and evidence communication boundaries, with blob references for large payloads.
- [ ] [parallel] r[molten.runtime_spine.core_purity] Keep core validation deterministic and free of filesystem, network, process, clock, and scripting access.
- [ ] [parallel] r[molten.runtime_spine.error_boundary] Define Snafu error types for core validation and adapter boundaries.

## Phase 2: Local runtime and configuration

- [ ] [serial] r[molten.runtime_spine.runtime_references] Map BEAM/OTP and Lunatic reference patterns to Molten actor lifecycle, supervision, mailbox, link/monitor, scheduling, and Wasm hostcall boundaries without claiming compatibility.
- [ ] [serial] r[molten.runtime_spine.local_dataspace] Add a local actor/dataspace prototype with native Rust actors exchanging envelopes.
- [ ] [parallel] r[molten.runtime_spine.config] Add Nickel-authored config that evaluates into typed startup configuration.
- [ ] [parallel] r[molten.runtime_spine.cli] Add Clap-based CLI commands for loading config and inspecting local runtime state.
- [ ] [parallel] r[molten.runtime_spine.observability] Add Tracing spans/events at runtime and adapter boundaries.

## Phase 3: Distributed content bridge

- [ ] [serial] r[molten.runtime_spine.remote_bridge] Add an Iroh gossip adapter for envelope-sized messages.
- [ ] [serial] r[molten.runtime_spine.blob_refs] Add an Iroh blobs adapter for content-addressed payload references.
- [ ] [parallel] r[molten.runtime_spine.docs_bridge] Add an Iroh docs adapter for replicated mutable document/state surfaces.
- [ ] [parallel] r[molten.runtime_spine.remote_admission] Reject remote envelopes whose blob references or canonical hashes do not validate.

## Phase 4: Execution adapters

- [ ] [serial] r[molten.runtime_spine.wasmtime_hostcalls] Add Wasmtime actor hostcalls for send, subscribe, blob_get, and blob_put.
- [ ] [parallel] r[molten.runtime_spine.wasi_capabilities] Add deny-by-default Wasmtime-WASI capability wiring for explicitly admitted host resources.
- [ ] [parallel] r[molten.runtime_spine.wit_components] Add WIT/component bindings and wasmparser-based module inspection before actor admission.
- [ ] [parallel] r[molten.runtime_spine.steel_orchestration] Add Steel orchestration APIs that operate through the runtime envelope spine.
- [ ] [serial] r[molten.runtime_spine.deny_by_default] Route adapter side effects through deny-by-default admission checks.

## Phase 5: Policy, evidence, and storage gates

- [ ] [serial] r[molten.runtime_spine.nickel_steel_contracts] Define Nickel-vs-Steel contract selection rules for applicable trust-boundary actions and require contract receipts before side effects.
- [ ] [serial] r[molten.runtime_spine.basalt_contracts] Add Basalt-backed UCAN/Nickel/Steel contract enforcement for capability-bearing runtime requests.
- [ ] [serial] r[molten.runtime_spine.policy_gate] Add Trellis-backed bounded admission predicates for capabilities, replay, and content integrity.
- [ ] [parallel] r[molten.runtime_spine.cairn_receipts] Validate action-envelope and lifecycle receipts through Cairn surfaces.
- [ ] [parallel] r[molten.runtime_spine.valence_evidence] Attach Octet/Valence evidence references to module and function provenance.
- [ ] [parallel] r[molten.runtime_spine.redb_store] Add a Redb store adapter for local metadata, receipt indexes, replay caches, and content-reference bookkeeping.
- [ ] [serial] r[molten.runtime_spine.integration_evidence] Add end-to-end tests showing config -> local route -> remote bridge -> policy/evidence admission boundaries.
- [ ] [parallel] r[molten.runtime_spine.property_tests] Add Hegel property-based tests for envelope, admission, and adapter invariants.
