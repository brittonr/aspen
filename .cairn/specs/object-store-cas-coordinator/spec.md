# Object Store Cas Coordinator Specification

## Purpose

Defines the `object-store-cas-coordinator` capability.

## Requirements

### Requirement: Ownership is a single CAS lease [r[aspen.cas.contract]]

In the durable-store-coordinated mode, ownership transfer MUST be a single compare-and-swap lease in a durable dataspace. There MUST be no fixed membership list.

#### Scenario: Node claims the current lease

- GIVEN a node claims an entity whose lease it holds
- WHEN the CAS decision runs
- THEN ownership MUST transfer only through one CAS lease
- AND no membership list MUST be required

#### Scenario: Lease does not match

- GIVEN a node claims an entity whose lease it does not hold
- WHEN the CAS decision runs
- THEN ownership MUST NOT change
- AND the claimant MUST NOT damage the entity state

### Requirement: The core decides from supplied values [r[aspen.cas.decision]]

The pure core MUST return acquire or reject from supplied current, expected, and proposed leases. It MUST read no store, clock, or network.

#### Scenario: Expected lease matches and proposed epoch advances

- GIVEN the expected owner and epoch match the current lease
- AND the proposed epoch advances under the replaceable-node posture
- WHEN the core decides
- THEN the disposition MUST be acquire
- AND the proposed owner and epoch MUST become the resulting lease

#### Scenario: Owner, epoch, or membership posture does not match

- GIVEN the expected lease is stale, the proposed epoch does not advance, or fixed membership is required
- WHEN the core decides
- THEN the disposition MUST be reject
- AND ownership MUST stay unchanged

### Requirement: Decisions do not prove runtime correctness [r[aspen.cas.boundary]]

A CAS lease decision MUST NOT prove runtime correctness, data integrity, or release readiness. A related CAS-arbiter reference MUST be bounded and non-parity.

The reference MUST NOT impose a consensus or vendor requirement.

#### Scenario: Decision is over-claimed

- GIVEN a decision or documentation claims correctness, integrity, or readiness
- WHEN boundary verification runs
- THEN that claim MUST fail verification
- AND the runtime and consumer boundaries MUST remain explicit

#### Scenario: Reference is treated as requirement

- GIVEN the WalTier pattern lacks a non-parity label or becomes a consensus mandate
- WHEN boundary verification runs
- THEN verification MUST fail
- AND the mechanism MUST stay an explicit extension-port option

### Requirement: Failure coverage remains explicit [r[aspen.cas.verification]]

Positive and negative fixtures MUST cover every declared lease and boundary.

#### Scenario: Complete focused matrix passes

- GIVEN contract, decision, fixtures, and documentation are complete
- WHEN focused package, workspace, Clippy, Cairn, and Nix verification runs
- THEN matching-acquisition and advanced-epoch inputs MUST pass
- AND each mismatched, stale, lost-lease, or fixed-membership input MUST fail as declared
