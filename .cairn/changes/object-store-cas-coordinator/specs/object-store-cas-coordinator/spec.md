# Object-Store CAS Coordinator Specification

## Purpose

Record a durable-store-coordinated ownership design contract for Aspen, using a single compare-and-swap lease instead of a membership protocol or consensus service.

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

The pure core MUST return acquire or reject from a supplied owner, claimant, and epoch pair. It MUST read no store, clock, or network.

#### Scenario: Owner matches and epoch advances

- GIVEN the claimed new owner matches the current owner and the epoch advances
- WHEN the core decides
- THEN the disposition MUST be acquire

#### Scenario: Owner or epoch does not match

- GIVEN the claimant does not match the owner or the epoch is stale
- WHEN the core decides
- THEN the disposition MUST be reject
- AND ownership MUST stay unchanged

### Requirement: Decisions do not prove runtime correctness [r[aspen.cas.boundary]]

A CAS lease decision MUST NOT be presented as proof of runtime correctness, data integrity, or release readiness.

#### Scenario: Decision is over-claimed

- GIVEN a decision or documentation claims correctness, integrity, or readiness
- WHEN boundary verification runs
- THEN that claim MUST fail verification
- AND the runtime and consumer boundaries MUST remain explicit

### Requirement: Failure coverage remains explicit [r[aspen.cas.verification]]

Positive and negative fixtures MUST cover every declared lease and boundary.

#### Scenario: Complete focused matrix passes

- GIVEN contract, decision, fixtures, and documentation are complete
- WHEN focused package, workspace, Clippy, Cairn, and Nix verification runs
- THEN matching-acquisition and advanced-epoch inputs MUST pass
- AND each mismatched, stale, lost-lease, or fixed-membership input MUST fail as declared
