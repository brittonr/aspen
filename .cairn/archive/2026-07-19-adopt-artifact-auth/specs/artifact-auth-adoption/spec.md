## ADDED Requirements

### Requirement: Molten adopts one immutable reviewed source

r[molten.artifact_auth_adoption.source] Molten MUST consume one immutable reviewed `artifact-auth` revision with aligned Cargo and Nix identities and MUST bind the Molten mapping profile and checked projection from that same revision before implementation or cutover.

#### Scenario: Source identity is admissible

- GIVEN Cargo, Nix, the mapping profile, and its checked projection resolve revision `799459346d5416fbd7b9f55840a7371441b55afa`
- WHEN Molten evaluates dependency admission
- THEN it SHALL reject floating, duplicate, mismatched, sibling-path, product-dependent, or license-incompatible source selections.

### Requirement: Molten retains runtime and authority semantics

r[molten.artifact_auth_adoption.authority] Molten MUST retain entropy, key generation/storage/signing, opaque handles, rotation writes, capability and federation authority, Preserves/Iroh transport, runtime policy, and evidence composition while treating standalone authentication as one bounded input.

#### Scenario: Authentication passes without runtime authority

- GIVEN a standalone signature and policy decision passes
- WHEN membership, capability, transport, runtime, deployment, or release admission runs
- THEN Molten MUST still require its product-owned checks and MUST NOT promote standalone success into product authority.

### Requirement: Cutover requires explained dual-run evidence

r[molten.artifact_auth_adoption.cutover] Molten MUST dual-run legacy and standalone paths over identical observations, classify every preimage, identity, decision, issue, and non-claim difference, reject unrelated-failure false parity, and preserve a bounded legacy rollback until standalone authority is explicitly admitted.

#### Scenario: Unexplained drift blocks cutover

- GIVEN any unexplained compatibility, currentness, or source-identity difference
- WHEN Molten evaluates cutover admission
- THEN the legacy path SHALL remain authoritative and the exact blocker SHALL be recorded without weakening runtime or authority gates.
