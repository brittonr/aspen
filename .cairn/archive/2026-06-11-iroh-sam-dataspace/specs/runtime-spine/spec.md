## ADDED Requirements

### Requirement: Remote dataspace envelopes
r[molten.iroh_sam_dataspace.envelope_dto] The system MUST represent remote SAM dataspace actions as canonical `remote-dataspace-envelope-v1` Preserves records carrying sender peer, sender actor, target peer, topic, operation, payload, content refs, capability refs, and evidence refs.

#### Scenario: Assertion envelope is canonical
r[molten.iroh_sam_dataspace.envelope_dto.assertion]
- GIVEN peer A actor `producer` wants to assert `<service-ready "db">` for peer B
- WHEN the remote dataspace adapter builds the envelope
- THEN the envelope operation is `assert`, its payload is the canonical Preserves assertion value, and its envelope ref is the Blake3 hash of the canonical Preserves bytes

### Requirement: Transport receipts for Iroh dataspace traffic
r[molten.iroh_sam_dataspace.transport_receipt_dto] The system MUST emit canonical `remote-dataspace-transport-receipt-v1` records for remote dataspace publish, deliver, and deny decisions.

#### Scenario: Publish receipt binds envelope and topic
r[molten.iroh_sam_dataspace.transport_receipt_dto.publish]
- GIVEN a valid remote dataspace envelope for topic `services`
- WHEN the Iroh transport adapter publishes the envelope
- THEN the transport receipt binds the envelope ref, transport name, source peer, target peer, topic, content refs, diagnostics, and checks

### Requirement: Transport identity is not authority
r[molten.iroh_sam_dataspace.transport_not_authority] The system MUST NOT treat Iroh endpoint identity, gossip topic membership, or blob possession as authority to act as an actor or mutate local dataspace state.

#### Scenario: Transport receipt is not enough for delivery admission
r[molten.iroh_sam_dataspace.transport_not_authority.not_enough]
- GIVEN a pass transport receipt for an envelope
- WHEN capability, policy, resource, or peer-bootstrap evidence is absent
- THEN the envelope MUST NOT be accepted as pass evidence for local side effects

### Requirement: Local Iroh-shaped deterministic adapter
r[molten.iroh_sam_dataspace.local_gossip_publish] The system MUST provide a deterministic local Iroh-shaped adapter for tests and repros that stores canonical envelope bytes under a local transport root and emits the same receipt shape as the live adapter.

#### Scenario: Local publish and deliver preserves envelope identity
r[molten.iroh_sam_dataspace.local_gossip_publish.roundtrip]
- GIVEN a canonical remote dataspace envelope
- WHEN it is published and delivered through the deterministic local Iroh-shaped adapter
- THEN the delivered envelope ref matches the published ref and the delivery receipt binds the same topic and peers

### Requirement: Content refs validate before delivery
r[molten.iroh_sam_dataspace.content_ref_validation] The system MUST validate declared remote dataspace content refs before delivering an envelope to local actors.

#### Scenario: Tampered content is rejected
r[molten.iroh_sam_dataspace.content_ref_validation.tampered]
- GIVEN a remote dataspace envelope that declares a blob content ref
- WHEN the local bytes for that content ref hash to a different value
- THEN delivery is denied before any actor observes the payload

### Requirement: Delivered envelopes apply through SAM turn semantics
r[molten.iroh_sam_dataspace.apply_assert_retract] Delivered remote assertion and retraction envelopes MUST apply through the local runtime turn boundary rather than mutating dataspace state directly.

#### Scenario: Remote assertion notifies local observer
r[molten.iroh_sam_dataspace.apply_assert_retract.observe]
- GIVEN peer B has a local observer for `<service-ready "db">`
- WHEN peer B delivers an admitted remote `assert` envelope from peer A actor `producer` with that payload
- THEN peer B records a normal assertion commit event owned by a remote actor identity and a normal assertion observed event for the local observer

### Requirement: Message and observe envelopes use the same runtime boundary
r[molten.iroh_sam_dataspace.apply_message_observe] Delivered remote message and observe envelopes MUST route through local message delivery and observer registration semantics.

#### Scenario: Remote observe registers an observer
r[molten.iroh_sam_dataspace.apply_message_observe.observe]
- GIVEN a delivered remote `observe` envelope with an exact Preserves pattern
- WHEN the envelope is applied locally after admission
- THEN the observer registration is represented as a normal runtime observe event under a remote actor identity

### Requirement: Recorded transport log for replay
r[molten.iroh_sam_dataspace.recorded_delivery_log] Evidence-bearing remote dataspace runs MUST either record the canonical transport delivery log for replay or be marked non-replayable and excluded from deterministic gates.

#### Scenario: Recorded replay does not consult live network
r[molten.iroh_sam_dataspace.recorded_delivery_log.replay]
- GIVEN a remote dataspace run with recorded envelope bytes, content refs, transport receipts, and admission receipts
- WHEN the run is replayed
- THEN replay uses the recorded transport log rather than live Iroh timing or peer availability

### Requirement: Live Iroh behind the same boundary
r[molten.iroh_sam_dataspace.live_iroh_gossip] Live `iroh-gossip` integration MUST use the same envelope, content-ref, admission, receipt, and replay boundaries as the deterministic local adapter.

#### Scenario: Live and local adapters share receipt shape
r[molten.iroh_sam_dataspace.live_iroh_gossip.same_shape]
- GIVEN the local adapter and live Iroh adapter publish equivalent envelopes
- WHEN their receipts are inspected
- THEN both receipts use `remote-dataspace-transport-receipt-v1` and differ only in transport/profile-specific refs allowed by policy
