# Molten World Commit Specification Delta

## Purpose

Add authenticated, generation-fenced, conflict-preserving mutable heads over immutable Molten world commits.

## ADDED Requirements

### Requirement: Head claims bind exact branch transitions

r[molten.world_heads.claim] Molten MUST define canonical detached head claims that bind branch identity, expected head, successor head, expected generation, successor generation, purpose, policy identity, and signer observations.

#### Scenario: Claim advances one branch

- GIVEN a claim names the current branch head and the next admitted generation
- WHEN claim validation runs
- THEN it MUST bind exactly one proposed transition without changing either commit identity

#### Scenario: Claim omits the expected head

- GIVEN a signed claim names only a successor
- WHEN validation runs
- THEN Molten MUST reject the claim as insufficient for compare-and-swap publication

### Requirement: Head publication is generation-fenced compare-and-swap

r[molten.world_heads.cas] Molten MUST recheck the current head and generation inside the mutation boundary. It MUST atomically record an admitted successor or leave the prior head current.

#### Scenario: Current state matches the plan

- GIVEN the persisted head and generation match the admitted claim
- WHEN the local transaction commits
- THEN the successor MUST become current with one transition operation record

#### Scenario: Another writer advanced first

- GIVEN the persisted generation changed after planning
- WHEN the stale transition enters the mutation boundary
- THEN Molten MUST deny publication and report expected and observed state

### Requirement: Statement authentication does not grant branch authority

r[molten.world_heads.authentication] Molten MUST use Artifact Auth only to authenticate exact statement bytes under supplied observations. Current Basalt, UCAN, signer-role, threshold, and durable currentness policy MUST still authorize every mutation.

#### Scenario: Signature and authority both pass

- GIVEN statement authentication passes and current branch authority admits the signer set
- WHEN transition admission runs
- THEN the claim MAY proceed to compare-and-swap publication

#### Scenario: Signature passes but authority denies

- GIVEN a cryptographically valid statement lacks current branch-mutation authority
- WHEN transition admission runs
- THEN publication MUST remain denied

### Requirement: Competing claims remain explicit conflicts

r[molten.world_heads.conflicts] Molten MUST preserve bounded competing valid claims for the same expected head and generation. It MUST NOT select a winner by wall-clock time, arrival order, lexical identity, or last-writer-wins behavior.

#### Scenario: Two authorized successors compete

- GIVEN two valid claims target the same branch state with different successors
- WHEN conflict classification runs
- THEN Molten MUST return an explicit conflict set and block automatic advance

### Requirement: Durable generations reject stale claims relative to intact state

r[molten.world_heads.rollback] Molten MUST reject claims with old, repeated, skipped, or contradictory generations unless an explicit fenced recovery policy admits repair. It MUST classify this protection as relative to the observed durable generation state. It MUST NOT claim whole-store rollback detection without an independent currentness or witness observation.

#### Scenario: Old signed claim is replayed

- GIVEN a previously valid claim names a generation below the durable current generation
- WHEN the claim is presented again
- THEN Molten MUST reject it as stale or replayed relative to the observed current state

#### Scenario: Durable generation cannot be observed

- GIVEN the head store cannot prove its current generation
- WHEN mutation admission runs
- THEN Molten MUST deny ordinary head movement instead of resetting generation state

#### Scenario: Head and generation store are rolled back together

- GIVEN local head and generation state are both restored to an older valid image and no independent currentness observation exists
- WHEN the local head protocol evaluates the image
- THEN Molten MUST report whole-store rollback detection as unproven
- AND it MUST NOT issue a strong rollback-resistance receipt

### Requirement: Branch-head verification covers conflicts and authority failures

r[molten.world_heads.verification] Molten MUST test valid advances, merge ancestry, stale state, rollback, replay, signer failures, currentness failures, competing claims, uncertain persistence, and bounded non-claims.

#### Scenario: Focused branch-head rail runs

- GIVEN positive and negative fixtures use the reviewed Choregraph and Artifact Auth cohorts
- WHEN the branch-head verification rail runs
- THEN it MUST report the supported local transition boundary without claiming distributed consensus
