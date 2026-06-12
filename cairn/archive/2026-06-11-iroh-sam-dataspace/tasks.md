## Phase 1: Canonical remote dataspace records

- [x] [serial] r[molten.iroh_sam_dataspace.envelope_dto] Define `remote-dataspace-envelope-v1` for message/assert/retract/observe actions with peer ids, actor ids, topic, payload, content refs, capability refs, and evidence refs.
- [x] [serial] r[molten.iroh_sam_dataspace.envelope_dto] Derive envelope refs only from canonical Preserves bytes and reject stale/tampered refs.
- [x] [parallel] r[molten.iroh_sam_dataspace.transport_receipt_dto] Define `remote-dataspace-transport-receipt-v1` for publish/deliver/deny decisions with transport, peer/topic bindings, content refs, diagnostics, and checks.
- [x] [parallel] r[molten.iroh_sam_dataspace.transport_receipt_dto] Classify remote dataspace envelopes and transport receipts in ledger/catalog surfaces.

## Phase 2: Deterministic local Iroh-shaped adapter

- [x] [serial] r[molten.iroh_sam_dataspace.local_gossip_publish] Add deterministic `iroh-local-gossip` publish of canonical envelope bytes under a local transport root.
- [x] [serial] r[molten.iroh_sam_dataspace.local_gossip_publish] Add deterministic local fetch/deliver that recomputes envelope refs, verifies topic/peer binding, and emits delivery receipt evidence.
- [x] [parallel] r[molten.iroh_sam_dataspace.content_ref_validation] Validate declared blob/chunk content refs before delivery and deny tampered/missing bytes.
- [x] [parallel] r[molten.iroh_sam_dataspace.transport_not_authority] Make transport receipts explicitly state that Iroh endpoint/topic membership is not actor authority.

## Phase 3: SAM runtime integration

- [x] [serial] r[molten.iroh_sam_dataspace.apply_assert_retract] Apply delivered `assert`/`retract` envelopes through the local runtime turn boundary, producing normal assertion and observer events.
- [x] [serial] r[molten.iroh_sam_dataspace.apply_message_observe] Apply delivered `message`/`observe` envelopes through local message delivery and observer registration semantics.
- [x] [parallel] r[molten.iroh_sam_dataspace.apply_assert_retract] Represent remote actor ownership without granting ambient local authority.
- [x] [parallel] r[molten.iroh_sam_dataspace.apply_message_observe] Bind delivered envelope refs and transport receipts into actor-scoped turn-journal context refs for later gate receipts.

## Phase 4: Admission and replay

- [x] [serial] r[molten.iroh_sam_dataspace.transport_not_authority] Require peer bootstrap/agreement refs before remote delivery can become pass evidence.
- [x] [serial] r[molten.iroh_sam_dataspace.transport_not_authority] Bind capability, policy, resource, and authority receipt refs before applying remote side effects.
- [x] [parallel] r[molten.iroh_sam_dataspace.recorded_delivery_log] Record delivery logs so deterministic replay does not depend on live network timing.
- [x] [parallel] r[molten.iroh_sam_dataspace.recorded_delivery_log] Mark unrecorded live transport runs non-replayable and exclude them from deterministic gates.

## Phase 5: Live Iroh and harness evidence

- [x] [serial] r[molten.iroh_sam_dataspace.live_iroh_gossip] Add real `iroh-gossip` publish/subscribe integration behind the same envelope/receipt boundary.
- [x] [serial] r[molten.iroh_sam_dataspace.live_iroh_gossip] Add a two-peer harness scenario: peer A asserts `service.ready`, peer B observes it remotely, and replay uses the recorded transport log.
- [x] [parallel] r[molten.iroh_sam_dataspace.transport_not_authority] Add gate receipt checks for envelope refs, transport receipts, bootstrap refs, authority refs, resource refs, and turn-journal refs.
- [x] [parallel] r[molten.iroh_sam_dataspace.content_ref_validation] Add negative tests for tampered envelope bytes, wrong topic, wrong peer, missing content refs, stale bootstrap evidence, and capability denial.
