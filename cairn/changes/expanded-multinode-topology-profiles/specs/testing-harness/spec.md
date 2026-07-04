## ADDED Requirements

### Requirement: Multinode topology profile matrix is explicit
r[molten.testing.multinode.topology_profile_matrix] Molten SHOULD declare a multinode topology profile matrix that names the topology id, node roles, member set, allowed links, evidence scope, and required receipt classes for each distributed scenario family.

#### Scenario: Topology profile is bound into run evidence
- GIVEN a multinode scenario using a pairwise transport, control quorum, restart/rejoin, or subscriber topology profile
- WHEN metadata or run receipts are generated
- THEN the receipts bind the topology profile id and topology ref
- AND the evidence scope remains distinct from the execution cost profile.

#### Scenario: Review distinguishes topology claims
- GIVEN two runs with the same command surface but different topology profiles
- WHEN reviewers inspect the canonical metadata
- THEN each run identifies the role shape it covered
- AND neither run can satisfy claims outside its declared topology profile.

### Requirement: Role and membership negative fixtures deny confusion
r[molten.testing.multinode.role_membership_negatives] Molten MUST reject multinode evidence that treats undeclared nodes, undeclared links, subscriber peers, transport-only peers, or missing quorum evidence as admitted control-plane membership or authority evidence.

#### Scenario: Subscriber is not promoted to voter
- GIVEN a topology profile that declares a subscriber peer outside the Raft voting member set
- WHEN a command attempts to use subscriber evidence as voter membership evidence
- THEN the gate denies before accepting the command as control-plane pass evidence
- AND diagnostics name the role or membership mismatch.

#### Scenario: Wrong topology cannot satisfy pass evidence
- GIVEN a receipt from a topology profile whose nodes or links differ from the scenario fixture under review
- WHEN the multinode gate evaluates the receipt
- THEN the gate rejects the receipt as stale or wrong-topology evidence.
