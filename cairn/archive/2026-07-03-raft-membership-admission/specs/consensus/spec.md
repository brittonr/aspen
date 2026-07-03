## ADDED Requirements

### Requirement: Raft membership changes have canonical records
r[molten.raft_membership_admission.model] Molten MUST define canonical Raft membership-change request, preflight receipt, and commit receipt records that bind group id, target peer/node, requested role, prior configuration ref, proposed configuration ref, authority refs, policy refs, resource refs, peer/session refs, source-gate/provenance refs, readiness refs, and diagnostics.

#### Scenario: Preflight receipt binds target and config
- GIVEN an operator requests adding a node to a Raft-backed control-plane group
- WHEN Molten emits a membership preflight receipt
- THEN the receipt names the group, target node, requested role, prior config ref, proposed config ref, evidence refs, decision, and diagnostics
- AND the receipt ref is derived from canonical Preserves bytes.

### Requirement: Raft membership is stronger than peer connectivity
r[molten.raft_membership_admission.stronger_than_peer] Molten MUST NOT admit Raft/control-plane membership from connected peer state, transport observations, gossip topic joins, docs namespace joins, protocol sessions, or job-pool joins alone.

#### Scenario: Connected peer cannot become voter implicitly
- GIVEN a peer session is connected and admitted for a gossip topic
- WHEN a Raft group membership check evaluates the peer
- THEN membership admission denies without a dedicated membership-change request and preflight receipt
- AND no Raft configuration entry is appended.

### Requirement: Membership preflight checks control-plane readiness
r[molten.raft_membership_admission.preflight_checks] Molten MUST require membership preflight to validate peer session scope, membership authority, policy, resources, source-gate/provenance, state-machine/schema compatibility, transport support, replay support, snapshot/log catch-up readiness, and operator evidence before mutation.

#### Scenario: Missing source-gate denies membership
- GIVEN a peer has valid transport and peer-session evidence but lacks current source-gate or provenance evidence for the control-plane artifact set
- WHEN membership preflight runs
- THEN the preflight decision is deny
- AND diagnostics identify source-gate or provenance evidence as missing.

### Requirement: Membership transitions preserve quorum safety
r[molten.raft_membership_admission.quorum_safety] Molten MUST bind Trellis/Raft predicate receipts for quorum preservation and configuration transition safety before any Raft membership change can commit.

#### Scenario: Unsafe removal denies before commit
- GIVEN a proposed membership change would remove voters without preserving quorum under the configured transition rule
- WHEN membership preflight or commit validation runs
- THEN the decision is deny
- AND no membership commit receipt claims the unsafe configuration.

### Requirement: Membership diagnostics distinguish evidence classes
r[molten.raft_membership_admission.diagnostics] Molten SHOULD diagnose peer connectivity, membership preflight, committed membership state, readiness evidence, and linearizable read evidence as separate state classes.

#### Scenario: Status shows connected but not member
- GIVEN a peer is connected but has no passing membership preflight or commit receipt
- WHEN membership diagnostics render status
- THEN they report the peer as connected but not a Raft member
- AND they name the missing membership evidence.

### Requirement: Membership CLI starts with dry-run preflight
r[molten.raft_membership_admission.cli_preflight] Molten SHOULD provide an operator dry-run membership preflight and readback summary before enabling or executing mutating membership changes.

#### Scenario: Dry-run does not mutate group
- GIVEN an operator runs membership preflight for a candidate node
- WHEN the command completes
- THEN it emits a preflight receipt and readback summary
- AND it does not append a Raft configuration entry or change group membership.

### Requirement: Membership tests cover positive and negative paths
r[molten.raft_membership_admission.positive_negative_tests] Molten SHOULD include positive membership preflight fixtures and negative tests for connected-peer-only, missing authority, missing source-gate, incompatible state-machine, stale snapshot, revoked peer, and quorum-safety denial.

#### Scenario: Connected-peer-only fixture denies
- GIVEN a fixture contains a connected peer session and live transport evidence but no membership-change request
- WHEN membership admission validates the fixture
- THEN admission denies
- AND diagnostics state that peer connectivity is not Raft membership.
