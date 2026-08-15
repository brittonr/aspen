## ADDED Requirements

### Requirement: Eventual propagation surfaces have manifests
r[molten.eventual_surface.manifest] Molten MUST define canonical eventual-surface manifests for gossip, docs, remote dataspace, and federation surfaces that claim eventual propagation or convergence semantics.

#### Scenario: Remote dataspace surface declares convergence inputs
- GIVEN a remote dataspace topic is configured as an eventual surface
- WHEN Molten serializes its surface manifest
- THEN the manifest names the scope, carrier, payload schema, idempotency key, merge law, retraction policy, anti-entropy policy, replay evidence requirement, and authority boundary.

### Requirement: Eventual convergence requires a merge law
r[molten.eventual_surface.merge_law] Molten MUST NOT claim eventual consistency for a propagation surface unless the surface declares a deterministic merge, duplicate, conflict, retraction, and tombstone law that can be validated over canonical inputs.

#### Scenario: Missing conflict resolver denies convergence claim
- GIVEN two peers publish concurrent state for the same eventual surface key
- WHEN the surface lacks a reviewed conflict-resolution law
- THEN Molten denies any convergence pass receipt
- AND diagnostics identify the missing merge or conflict law.

### Requirement: Eventual propagation is not consensus
r[molten.eventual_surface.not_consensus] Molten MUST label gossip, docs, remote dataspace, and federation propagation evidence as non-linearizable and non-authoritative unless a separate Raft/control-plane receipt admits the resulting operation.

#### Scenario: Gossip delivery cannot replace Raft commit
- GIVEN a gossip envelope requests a control-plane mutation
- WHEN the envelope is delivered to a peer
- THEN delivery evidence alone cannot claim the mutation is committed
- AND a separate Raft/control-plane commit receipt remains required for strong state.

### Requirement: Deterministic replay requires recorded propagation evidence
r[molten.eventual_surface.replay_boundary] Molten MUST require recorded delivery logs, snapshots, anti-entropy receipts, or equivalent canonical evidence before live eventual propagation can satisfy deterministic pass gates.

#### Scenario: Unrecorded live timing is diagnostic only
- GIVEN a live Iroh gossip exchange succeeds but no delivery log or replayable snapshot is recorded
- WHEN a deterministic gate evaluates the exchange
- THEN the gate treats the live timing as diagnostic only
- AND denies pass evidence that depends on replaying the unrecorded timing.

### Requirement: Eventual surface diagnostics distinguish state classes
r[molten.eventual_surface.diagnostics] Molten SHOULD report whether an observed value is merely delivered, merged into an eventual surface, replayable, admitted by authority/policy, or committed by consensus.

#### Scenario: Delivered but not admitted is visible
- GIVEN an envelope has a transport delivery receipt but lacks authority or policy admission
- WHEN diagnostics render the eventual surface status
- THEN they report the envelope as delivered but not admitted
- AND they do not mark it as authoritative state.

### Requirement: Eventual propagation tests cover convergence and denials
r[molten.eventual_surface.positive_negative_tests] Molten SHOULD include positive convergence fixtures and negative fixtures for missing merge laws, unresolved concurrent conflicts, stale tombstones, unrecorded live timing, and attempts to use propagation evidence as authority.

#### Scenario: Propagation-as-authority fixture denies
- GIVEN a fixture supplies only a gossip publish and deliver receipt for a side-effecting operation
- WHEN the eventual surface validator evaluates authority state
- THEN validation denies authority
- AND diagnostics say propagation evidence is not an authority grant.
