# Molten World Commit Specification Delta

## Purpose

Add independent witness currentness for strong world-head rollback resistance without transferring provider, authentication, or branch authority into Molten.

## ADDED Requirements

### Requirement: Head currentness profiles state their assurance boundary

r[molten.world_witness.profiles] Molten MUST distinguish local-generation and independent-witness currentness profiles. A local profile MUST NOT satisfy policy that requires detection of rollback affecting both the head and its local generation store.

#### Scenario: Local durable state is intact

- GIVEN an old claim is presented below the observed local generation
- WHEN local-profile admission runs
- THEN Molten MUST reject the stale claim
- AND the receipt MUST scope the result to intact local currentness state

#### Scenario: Strong policy receives only local state

- GIVEN branch policy requires independent witnessing and no admitted witness observation exists
- WHEN head finalization runs
- THEN Molten MUST deny the transition

### Requirement: Witness observations bind exact provider and head facts

r[molten.world_witness.currentness] Molten MUST validate provider identity, branch identity, head-claim identity, generation, signed checkpoint, inclusion, consistency predecessor, quorum, and highest admitted consumer state before accepting independent currentness.

#### Scenario: Complete witness chain advances

- GIVEN an admitted provider proves inclusion of the exact claim and consistency from the highest admitted checkpoint
- WHEN currentness validation runs
- THEN Molten MAY admit the observation for final head publication

#### Scenario: Provider returns a proof for another claim

- GIVEN inclusion is valid for a different branch, generation, or head-claim identity
- WHEN currentness validation runs
- THEN Molten MUST reject the observation as domain substitution

### Requirement: Strong head publication is witness-first and locally fenced

r[molten.world_witness.finalization] Molten MUST durably stage a claim before external append. It MUST publish a strong-profile head only after admitted witness observations return and all local head, generation, policy, authority, staged-claim, and witness-state facts pass inside the final mutation boundary.

#### Scenario: Witness passes and local state remains current

- GIVEN the staged claim is witnessed and all final local rechecks match
- WHEN the local transaction commits
- THEN the head, generation, highest witness state, and transition record MUST become current together

#### Scenario: Another writer advances after witnessing

- GIVEN the provider witnessed a staged claim but the local head changed before finalization
- WHEN final compare-and-swap runs
- THEN Molten MUST preserve the witnessed claim as superseded or conflicting
- AND it MUST NOT move the head from stale expected state

### Requirement: Witness uncertainty remains explicit

r[molten.world_witness.reconciliation] Timeout or disconnect after possible provider append or local commit MUST produce an uncertain result. Reconciliation MUST inspect exact operation identities and classify finalize, already-complete, superseded, conflict, retryable, denied, or manual-review outcomes.

#### Scenario: Append response is lost

- GIVEN the provider may have appended the claim but the response is unavailable
- WHEN the shell handles the timeout
- THEN it MUST return uncertain
- AND it MUST NOT submit an unbound duplicate append as a new operation

### Requirement: Witness evidence does not grant authority or truth

r[molten.world_witness.provider_boundary] A valid witness observation MUST prove only bounded inclusion and currentness facts under the selected provider contract. It MUST NOT grant branch authority or prove commit truth, provider honesty, global consensus, release eligibility, or external effect completion.

#### Scenario: Witness passes but authority denies

- GIVEN valid inclusion and consistency observations exist but current branch authority denies the signer set
- WHEN final admission runs
- THEN head movement MUST remain denied

### Requirement: Witness verification covers rollback and provider failure

r[molten.world_witness.verification] Molten MUST test valid witnessed advances, whole-store rollback, stale checkpoints, invalid inclusion, consistency failure, forks, quorum failure, provider substitution, unavailable providers, uncertainty, missing consumer state, and authority overclaims.

#### Scenario: Focused witness rail runs

- GIVEN positive and negative fixtures use the reviewed provider contract
- WHEN the witness verification rail runs
- THEN it MUST report every supported assurance profile and its bounded non-claims

### Requirement: Witness receipts preserve separate decision roles

r[molten.world_witness.receipts] Molten MUST emit detached receipts that separately identify statement authentication, branch authorization, witness currentness, local persistence, reconciliation, and unsupported claims.

#### Scenario: Receipt collapses witness and authority

- GIVEN a receipt states that witness inclusion authorized a branch mutation
- WHEN receipt validation runs
- THEN Molten MUST reject the receipt as an authority overclaim
