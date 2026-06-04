## Phase 1: Closure descriptors

- [ ] [serial] r[molten.remote_sync.closure_descriptor] Define remote dependency closure descriptors with root artifact id, dependency set or closure hash, kind hints, effect manifest refs, policy refs, and replay nonce.
- [ ] [serial] r[molten.remote_sync.missing_set] Compute the receiver's missing artifact set from the local registry and a closure descriptor.
- [ ] [parallel] r[molten.remote_sync.no_mobile_closures] Document that remote sync moves admitted artifacts and arguments, not arbitrary live heap closures.
- [ ] [parallel] r[molten.remote_sync.transport_neutral] Keep sync descriptors as canonical envelopes independent of Iroh-specific transport details.

## Phase 2: Fetch, verify, and install

- [ ] [serial] r[molten.remote_sync.iroh_fetch] Fetch missing immutable artifact payloads through Iroh blobs using content ids or tickets.
- [ ] [serial] r[molten.remote_sync.hash_verify] Verify fetched artifact bytes against domain-separated canonical artifact ids before staging.
- [ ] [serial] r[molten.remote_sync.install_admission] Gate staged dependency closure installation through local Nickel/Basalt/Trellis policy.
- [ ] [parallel] r[molten.remote_sync.cache_index] Record verified and admitted artifacts in a local cache index with source, evidence, last-use, and pinning metadata.
- [ ] [parallel] r[molten.remote_sync.fetch_receipts] Emit Cairn receipts for missing-set calculation, fetch source, hash verification, and install admission.

## Phase 3: Remote execution

- [ ] [serial] r[molten.remote_sync.execution_envelope] Define remote execution envelopes with execution id, artifact id, entrypoint, args, effect manifest, handler profile, capabilities, and evidence refs.
- [ ] [serial] r[molten.remote_sync.handler_binding] Bind remote execution to admitted effect handlers on the target peer before starting execution.
- [ ] [serial] r[molten.remote_sync.result_envelope] Return canonical result or structured failure envelopes with execution receipts and trace refs.
- [ ] [parallel] r[molten.remote_sync.loopback_executor] Add a loopback/local executor test target before real multi-peer scheduling.

## Phase 4: Safety and tests

- [ ] [serial] r[molten.remote_sync.incomplete_reject] Reject execution when dependency closure, policy admission, or handler binding is incomplete.
- [ ] [serial] r[molten.remote_sync.replay_bounds] Add bounded replay/session checks for remote install and execution requests.
- [ ] [parallel] r[molten.remote_sync.gc_safety] Track pins from active executions, protocols, durable storage refs, receipts, and metadata before cache eviction.
- [ ] [parallel] r[molten.remote_sync.property_tests] Add Hegel property tests for closure determinism, missing-set correctness, hash verification, and cache pin invariants.
