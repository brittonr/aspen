## ADDED Requirements

### Requirement: Nickel array contracts express uniqueness and bounds
r[molten.nickel_array_invariants.shared_array_helpers] Repository-owned Nickel contract modules SHOULD use shared helper contracts for array uniqueness, non-empty arrays, maximum lengths, required members, and unique BLAKE3 ref lists when those invariants are part of the reviewed domain.

#### Scenario: Duplicate reviewed ref fails export
- GIVEN a Nickel fixture whose field is declared as a unique evidence-ref array
- WHEN the fixture repeats the same BLAKE3 ref
- THEN Nickel export fails before generated JSON can be refreshed.

### Requirement: Array helper diagnostics identify the invariant
r[molten.nickel_array_invariants.helper_diagnostics] Repository-owned Nickel contracts SHOULD apply named helper predicates or targeted fixtures so duplicate, missing-member, non-empty, and bound failures identify the intended array invariant under test.

#### Scenario: Duplicate descriptor fixture identifies uniqueness
- GIVEN a negative fixture with duplicate plugin descriptor identities
- WHEN Nickel export evaluates the contract
- THEN the failure is associated with the descriptor uniqueness invariant
- AND generated evidence is not refreshed.

### Requirement: Production, peer, and multinode arrays reject ambiguity
r[molten.nickel_array_invariants.production_peer_multinode] Production profile, peer profile, and multinode scenario contracts MUST reject duplicate or contradictory array values where duplicates would make adapter membership, peer identity, artifact kinds, receipt refs, variance refs, or caveats ambiguous.

#### Scenario: Duplicate peer ref denies
- GIVEN a peer profile export with two profiles using the same peer ref
- WHEN Nickel evaluates the fixture
- THEN export fails with a duplicate-identity invariant.

### Requirement: Plugin contract arrays reject duplicate reviewed identities
r[molten.nickel_array_invariants.plugin_arrays] Plugin extension contracts and plugin capability grants MUST reject duplicate lifecycle callbacks, duplicate hostcall descriptor identities, duplicate required refs, and oversized evidence arrays where those fields are reviewed as sets.

#### Scenario: Duplicate lifecycle callback denies
- GIVEN a plugin extension contract fixture with the same lifecycle callback listed twice
- WHEN Nickel evaluates the fixture
- THEN export fails before the plugin contract can be converted to generated evidence.

### Requirement: Cairn policy arrays reject duplicate reviewed ids
r[molten.nickel_array_invariants.policy_arrays] Cairn policy contracts SHOULD use shared array helpers where schema ids, marker ids, marker tokens, replay ids, surface ids, receipt schema commands, or other reviewed policy tokens must be unique.

#### Scenario: Duplicate marker token fails export
- GIVEN a Cairn policy fixture with duplicate task marker tokens
- WHEN Nickel export evaluates the policy contract
- THEN export fails before generated policy JSON can be refreshed.

### Requirement: Array invariant failures have negative fixtures
r[molten.nickel_array_invariants.negative_arrays] Every newly tightened Nickel array invariant SHOULD have a negative fixture that demonstrates the intended duplicate, oversize, missing-member, or contradictory-list failure.

#### Scenario: Oversized array fixture fails
- GIVEN a contract field with a configured maximum array length
- WHEN a negative fixture exceeds that length
- THEN the fixture fails export and identifies the array invariant under test.

### Requirement: Nickel array tightening remains authoring-time only
r[molten.nickel_array_invariants.runtime_boundary] Nickel array invariant contracts MUST remain authoring-time fixture validation and MUST NOT replace runtime Preserves parsing, authority gates, policy gates, resource gates, provenance gates, retention gates, or execution gates.

#### Scenario: Valid export still requires runtime admission
- GIVEN a Nickel fixture exports successfully after array invariant validation
- WHEN runtime admission consumes the generated evidence
- THEN runtime still requires the subsystem's canonical receipt and semantic gates.
