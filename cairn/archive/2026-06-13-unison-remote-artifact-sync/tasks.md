## Phase 1: Closure descriptors

- [x] [serial] r[molten.remote_sync.closure_descriptor] Define remote dependency closure descriptors with root artifact id, dependency set or closure hash, kind hints, effect manifest refs, policy refs, and replay nonce.
- [x] [serial] r[molten.remote_sync.missing_set] Compute the receiver's missing artifact set from the local registry and a closure descriptor.
- [x] [parallel] r[molten.remote_sync.no_mobile_closures] Document and enforce that remote sync moves admitted artifacts and arguments, not arbitrary live heap closures.
- [x] [parallel] r[molten.remote_sync.transport_neutral] Keep sync descriptors as canonical envelopes independent of Iroh-specific transport details.

## Phase 2: Fetch, verify, and install

- [x] [serial] r[molten.remote_sync.iroh_fetch] Fetch missing immutable artifact payloads through admitted Iroh/blob or chunk-store boundaries where remote content transport is used; loopback uses the same ids without network.
- [x] [serial] r[molten.remote_sync.hash_verify] Verify fetched or loopback-copied artifact bytes against domain-separated canonical artifact ids before staging.
- [x] [serial] r[molten.remote_sync.install_admission] Gate staged dependency closure installation through local policy/provenance/source-gate/capability/resource admission.
- [x] [parallel] r[molten.remote_sync.cache_index] Record verified and admitted artifacts in local registry, ledger, or cache indexes with source, evidence, last-use, and pinning metadata.
- [x] [parallel] r[molten.remote_sync.fetch_receipts] Emit canonical receipts for missing-set calculation, fetch/source, hash verification, provenance, and install admission.

## Phase 3: Remote execution

- [x] [serial] r[molten.remote_sync.execution_envelope] Define remote execution envelopes with execution id, artifact/stage ids, entrypoint, args, effect manifest, handler profile, capabilities, and evidence refs.
- [x] [serial] r[molten.remote_sync.handler_binding] Bind remote execution to admitted effect handlers or target execution profiles before starting execution.
- [x] [serial] r[molten.remote_sync.result_envelope] Return canonical result or structured failure envelopes with execution receipts and trace/output refs.
- [x] [parallel] r[molten.remote_sync.loopback_executor] Add a loopback/local executor test target before real multi-peer scheduling.

## Phase 4: Safety and tests

- [x] [serial] r[molten.remote_sync.incomplete_reject] Reject execution when dependency closure, policy admission, source-gate evidence, resource evidence, or handler binding is incomplete.
- [x] [serial] r[molten.remote_sync.replay_bounds] Add bounded replay/session/operation-id checks for remote install and execution requests.
- [x] [parallel] r[molten.remote_sync.gc_safety] Track pins from active executions, protocols, durable storage refs, receipts, and metadata before cache eviction.
- [x] [parallel] r[molten.remote_sync.property_tests] Add Hegel property tests for closure determinism, missing-set correctness, hash verification, replay bounds, and cache pin invariants.
