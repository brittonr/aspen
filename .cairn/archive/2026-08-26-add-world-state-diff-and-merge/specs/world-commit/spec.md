# Molten World Commit Specification Delta

## Purpose

Add deterministic world comparison and narrow semantic merge without treating all runtime roots as mergeable data.

## ADDED Requirements

### Requirement: World diff reports every typed root conservatively

r[molten.world_merge.diff] Molten MUST compare typed world roots deterministically and classify each root as equal, changed, absent, unavailable, incompatible, or excluded by profile.

#### Scenario: Two commits differ in durable state

- GIVEN both commits have complete comparable roots and only durable state differs
- WHEN world diff runs
- THEN the report MUST identify the durable-state change and preserve equal classifications for unchanged roots

#### Scenario: Referenced object is unavailable

- GIVEN one root object cannot be loaded within the declared observation
- WHEN world diff runs
- THEN the report MUST classify that root as unavailable instead of equal or changed

### Requirement: Merge admission is typed and default-deny

r[molten.world_merge.admission] Molten MUST require an exact common ancestor, declared source heads, complete inputs, admitted schemas, and a closed merge mode. Tasks, scheduler state, authority state, effect attempts, external observations, and opaque machine snapshots MUST remain non-mergeable by default.

#### Scenario: One durable-value side changed

- GIVEN one side equals the base and the other side has a compatible durable-value root
- WHEN ancestor-replacement admission runs
- THEN the changed root MAY enter merge planning

#### Scenario: Candidate requests task-root merge

- GIVEN divergent task roots have no accepted root-specific merge profile
- WHEN merge admission runs
- THEN Molten MUST deny the merge before output publication

### Requirement: Merge handlers are exact pure bounded artifacts

r[molten.world_merge.handlers] Molten MUST bind application handlers by exact immutable behavior, schema, bound, and policy identities. Handlers MUST operate only on already-loaded canonical values and MUST NOT perform I/O, time, entropy, authority lookup, or effects.

#### Scenario: Exact handler returns a value

- GIVEN the handler identity and input schemas match the admitted profile
- WHEN the pure handler returns a bounded compatible value
- THEN the value MAY enter result planning

#### Scenario: Handler requests an external effect

- GIVEN a handler requires storage, network, clock, entropy, or authority access
- WHEN handler admission runs
- THEN Molten MUST reject it from the pure merge boundary

### Requirement: Unresolved conflicts are explicit durable results

r[molten.world_merge.conflicts] Molten MUST return deterministic typed conflict artifacts for unresolved semantic differences. It MUST NOT select winners through ordering, timestamps, or content identity.

#### Scenario: Both sides change one key differently

- GIVEN a keyed durable value differs from the base on both sides with incompatible values
- WHEN keyed merge runs
- THEN the result MUST contain one typed conflict and no merged root

### Requirement: Successful merge publishes one new causal commit

r[molten.world_merge.result] Molten MUST persist every result root before publishing a merge commit. The commit MUST name all declared source heads as parents, and unresolved conflicts MUST leave heads unchanged.

#### Scenario: Merge has no conflicts

- GIVEN every output root is durable and the result plan remains current
- WHEN merge publication succeeds
- THEN Molten MUST publish one new commit with the declared parent set

#### Scenario: Output publication fails

- GIVEN one merged root cannot be durably published
- WHEN merge publication runs
- THEN Molten MUST NOT publish a successful merge commit or move a branch head

### Requirement: Merge verification includes unsafe runtime roots

r[molten.world_merge.verification] Molten MUST test supported clean merges and negative ancestry, schema, migration, handler, conflict, bound, runtime-root, and partial-publication cases.

#### Scenario: Focused merge rail runs

- GIVEN positive and negative fixtures use reviewed history and schema cohorts
- WHEN the merge verification rail runs
- THEN it MUST report supported modes and all default-denied root classes
