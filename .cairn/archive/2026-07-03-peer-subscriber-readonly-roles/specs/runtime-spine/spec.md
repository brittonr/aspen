## ADDED Requirements

### Requirement: Subscriber projections enforce egress policy
r[molten.peer_subscriber.egress_policy] Molten MUST validate subscriber projections through egress policy, redaction profile, deny-sensitive content handling, resource bounds, replayability metadata, and diagnostics before delivery.

#### Scenario: Secret-bearing projection is denied or redacted
- GIVEN a subscriber projection contains private refs, secret material, revoked content, or capability-bearing payloads outside the grant scope
- WHEN egress validation runs
- THEN Molten denies delivery or emits a redacted projection according to the configured policy
- AND the projection receipt records the filter/redaction decision.

### Requirement: Subscriber projections remain propagation evidence
r[molten.peer_subscriber.surface_projection] Molten MUST treat subscriber projection receipts as eventual propagation/readback evidence and MUST NOT let them claim consensus, authority, provenance, source-gate, retention, execution, or destructive-operation trust.

#### Scenario: Projection delivery cannot commit state
- GIVEN a subscriber receives a projected dataspace assertion
- WHEN diagnostics evaluate the projection receipt
- THEN the value is marked as delivered/readback evidence only
- AND no control-plane state is considered committed without separate consensus evidence.

### Requirement: Subscriber validation is reproducible
r[molten.peer_subscriber.validation] Molten SHOULD validate subscriber/read-only roles with focused subscriber tests, peer/session tests, eventual surface tests, federation tests, consensus boundary tests, formatting, and Cairn validation before archiving.

#### Scenario: Egress regression fails validation
- GIVEN a regression delivers a secret-bearing projection without redaction or denial
- WHEN focused subscriber validation runs
- THEN the negative egress fixture fails
- AND the change cannot complete until the egress policy denial or redaction is restored.
