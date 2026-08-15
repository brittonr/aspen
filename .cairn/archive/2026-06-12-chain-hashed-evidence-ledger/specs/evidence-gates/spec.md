# Evidence Gates Delta: chain-hashed evidence ledger

## ADDED Requirements

### Requirement: Chain links preserve payload identity
r[molten.evidence.chain_hashing.link_model] Evidence chain links MUST be canonical Preserves artifacts whose refs are computed from the link bytes while preserving the canonical refs of linked payload artifacts.

#### Scenario: Linking a gate receipt does not change the receipt ref
r[molten.evidence.chain_hashing.link_model.preserve_payload]
- GIVEN a canonical `<gate-receipt-v1 ...>` with a known receipt ref
- WHEN a chain link names that receipt as its payload
- THEN the link has its own canonical link ref
- AND the payload ref inside the link equals the original gate receipt ref

#### Scenario: Link identity is stable
r[molten.evidence.chain_hashing.link_identity.stable]
- GIVEN the same chain scope, sequence, previous ref, payload ref, context refs, producer refs, and checks
- WHEN the link is encoded canonically twice
- THEN both encodings produce the same link ref

### Requirement: Chain appends are scoped and monotonic
r[molten.evidence.chain_hashing.genesis_append] Chain append validation MUST enforce genesis shape, same-scope previous-link binding, and monotonic sequence numbers within a chain scope/id/epoch.

#### Scenario: Genesis starts a scoped chain
r[molten.evidence.chain_hashing.genesis_append.genesis]
- GIVEN a chain link with sequence `0`
- WHEN the link has no previous ref and has an admitted chain scope/id/epoch
- THEN append validation accepts it as a genesis link

#### Scenario: Non-genesis links bind the previous link
r[molten.evidence.chain_hashing.genesis_append.previous]
- GIVEN an existing link at sequence `41` for a chain scope/id/epoch
- WHEN a new link for the same chain names the existing link as `prev` and uses sequence `42`
- THEN append validation accepts the continuity check

#### Scenario: Sequence gaps are rejected
r[molten.evidence.chain_hashing.genesis_append.gap]
- GIVEN an existing link at sequence `41`
- WHEN a new link for the same chain names that link as `prev` but uses sequence `43`
- THEN append validation fails closed with a gap diagnostic

### Requirement: Chain verification detects tampering and forks
r[molten.evidence.chain_hashing.verify_receipts] Chain verification MUST emit canonical verification receipts that identify verified links, payload refs, accepted anchor/head refs, and any tamper, gap, stale-head, missing-payload, or fork diagnostics.

#### Scenario: Previous-ref tampering is rejected
r[molten.evidence.chain_hashing.verify_receipts.prev_tamper]
- GIVEN a chain segment from an accepted anchor to a claimed head
- WHEN a link in the segment names a previous ref that does not match the prior verified link
- THEN verification rejects the segment
- AND the verification receipt names the first divergent link

#### Scenario: Fork is rejected under no-fork policy
r[molten.evidence.chain_hashing.verify_receipts.fork]
- GIVEN two links in the same chain scope/id/epoch that both name the same previous link
- WHEN the chain policy requires no forks
- THEN verification rejects the claimed head
- AND emits fork evidence naming both child link refs

#### Scenario: Diagnostic profile can retain fork evidence
r[molten.evidence.chain_hashing.verify_receipts.fork_diagnostic]
- GIVEN a detected fork
- WHEN the active evidence profile is diagnostic-only
- THEN the ledger may retain both fork links and the fork diagnostic receipt
- AND those artifacts do not satisfy production pass evidence gates

### Requirement: Gate profiles may require chain continuity
r[molten.evidence.chain_hashing.gate_receipts] Evidence gates SHOULD be able to require selected pass artifacts to descend from trusted chain anchors or fresh control-plane checkpoints.

#### Scenario: Production gate requires anchored receipt
r[molten.evidence.chain_hashing.gate_receipts.anchor_required]
- GIVEN a production evidence profile that requires chain-hashed receipts
- WHEN a valid gate receipt is not reachable from an accepted chain anchor or checkpoint
- THEN the production gate rejects the receipt as insufficient pass evidence

#### Scenario: Stale head is rejected
r[molten.evidence.chain_hashing.gate_receipts.stale_head]
- GIVEN a verified chain segment that descends from a trusted anchor
- AND a control-plane checkpoint names a newer accepted head for the same chain
- WHEN a gate attempts to use the older head without an admitted historical policy
- THEN the gate rejects the evidence as stale

### Requirement: Trellis predicates bound chain continuity
r[molten.evidence.chain_hashing.trellis_append_predicates] The system SHOULD provide Trellis-backed bounded predicates for chain genesis validity, append validity, no-gap continuity, no-fork policy, anchor descent, and checkpoint range coverage.

#### Scenario: Trellis append predicate agrees with pure validation
r[molten.evidence.chain_hashing.trellis_append_predicates.append]
- GIVEN a bounded previous link summary and candidate link summary
- WHEN pure chain validation accepts the append
- THEN the Trellis append predicate also accepts the append

#### Scenario: Trellis no-fork predicate rejects duplicate children
r[molten.evidence.chain_hashing.trellis_append_predicates.no_fork]
- GIVEN a bounded segment containing two accepted children for one parent under no-fork policy
- WHEN the Trellis no-fork predicate evaluates the segment
- THEN it rejects the segment and names the parent/child summaries

### Requirement: Chain hashing is not global actor ordering
r[molten.evidence.chain_hashing.no_global_chain] Chain hashing MUST NOT require ordinary actor messages or unrelated actor turns to depend on one global chain head.

#### Scenario: Independent turn journals can advance concurrently
r[molten.evidence.chain_hashing.no_global_chain.concurrent_turns]
- GIVEN two unrelated actors with independent turn-journal chain scopes
- WHEN each actor commits an admitted turn
- THEN each turn journal advances under its own chain head
- AND neither turn requires the other's head as previous evidence
