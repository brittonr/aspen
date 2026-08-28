# Molten World Commit Specification Delta

## Purpose

Add a pinned, authority-neutral DoltLite oracle for bounded semantic-state compatibility and differential evidence.

## ADDED Requirements

### Requirement: Oracle source and build identity are exact

r[molten.world_state_oracle.source] The DoltLite oracle MUST bind the exact upstream source revision, imported scope, applicable notices, build inputs, feature set, backend format, and adapter version. Remote support MUST remain disabled for this pilot.

#### Scenario: Oracle source is admitted

- GIVEN the source revision, license material, build inputs, and disabled-remote profile match the reviewed cohort
- WHEN oracle admission runs
- THEN Molten MAY plan a disposable test execution

#### Scenario: Remote support is enabled

- GIVEN the candidate build enables DoltLite remote behavior
- WHEN oracle admission runs
- THEN Molten MUST reject the build before test execution

### Requirement: Oracle stays behind a test-owned boundary

r[molten.world_state_oracle.boundary] Molten MUST expose DoltLite only through a test-owned semantic-state oracle port. Molten cores MUST NOT depend on SQLite types, DoltLite types, file paths, process state, or hidden current-branch state.

#### Scenario: Core receives normalized facts

- GIVEN the shell executes one admitted DoltLite operation
- WHEN it returns the result to comparison logic
- THEN the result MUST use Molten-owned semantic observations and explicit infrastructure errors

#### Scenario: Vendor type reaches the core

- GIVEN an adapter attempts to pass a SQLite handle, row, error, or branch-global into a Molten core
- WHEN the architecture gate runs
- THEN the gate MUST reject the dependency direction

### Requirement: Oracle observations are canonical and identity-separated

r[molten.world_state_oracle.observations] The oracle MUST use explicit primary keys and deterministic ordering. It MUST emit canonical ordered semantic observations with a Molten-owned BLAKE3 identity. DoltLite object identifiers MUST remain backend-local evidence.

#### Scenario: Two backends agree semantically

- GIVEN Molten and DoltLite produce the same ordered keys, values, and operation outcomes
- WHEN differential comparison runs
- THEN it MUST report bounded semantic agreement without asserting root-format equality

#### Scenario: Schema depends on rowid

- GIVEN an oracle schema or case depends on implicit SQLite rowid identity
- WHEN fixture admission runs
- THEN Molten MUST reject the case as noncanonical

### Requirement: Compatibility differences are typed and ratcheted

r[molten.world_state_oracle.compatibility] The repository MUST maintain a Nickel compatibility ledger with closed statuses `compatible`, `adapted`, `intentional`, `unsupported`, and `engine-gap`. Every row MUST bind source contract, evidence, fixture, and explanation. Each `unsupported` or `engine-gap` row MUST also bind a tracked issue and negative fixture.

#### Scenario: Exception count does not increase

- GIVEN all rows have valid evidence and exception counts do not exceed policy maxima
- WHEN the compatibility gate runs
- THEN it MUST report the exact status totals

#### Scenario: Unsupported row lacks a negative fixture

- GIVEN one unsupported or engine-gap row omits its issue or negative fixture
- WHEN the compatibility gate runs
- THEN it MUST fail before acceptance

### Requirement: Oracle cases cover branch, concurrency, format, and GC behavior

r[molten.world_state_oracle.behavior] The oracle suite MUST cover history-independent primary-key state, detached reads, branch isolation, stale-snapshot denial, compare-and-advance races, reader-safe GC, exact format rejection, and serialization behavior.

#### Scenario: Different mutation histories converge

- GIVEN two admitted operation sequences produce the same canonical primary-key state
- WHEN each sequence commits under the same oracle profile
- THEN the oracle MUST report equal backend roots for that pinned cohort

#### Scenario: Stale reader attempts a write upgrade

- GIVEN a reader holds an older snapshot and another writer advances the branch
- WHEN the stale session attempts to write without refresh
- THEN the oracle MUST report the expected stale-snapshot denial class

### Requirement: Oracle verification preserves Molten ownership and non-claims

r[molten.world_state_oracle.verification] The pilot MUST test positive and negative cases for source admission, canonical observations, branch behavior, concurrency, GC, format handling, ledger enforcement, and claim boundaries. It MUST keep complete-world atomicity, durable conflicts, typed merge, authority, effect release, retention policy, and stack-global identities under Molten ownership.

#### Scenario: Oracle behavior conflicts with Molten policy

- GIVEN DoltLite keeps a conflict only inside a transaction or cannot atomically write multiple file-backed databases
- WHEN the compatibility result is classified
- THEN Molten MUST record an intentional difference and MUST NOT weaken its world-commit contract

#### Scenario: Negative corpus is incomplete

- GIVEN fixtures omit stale-writer, wrong-format, missing-pin, malformed, unsupported, or overclaim cases
- WHEN verification coverage is evaluated
- THEN the change MUST remain incomplete
