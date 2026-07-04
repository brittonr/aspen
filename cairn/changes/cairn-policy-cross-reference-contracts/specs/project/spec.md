# Project Delta: Cairn policy cross-reference contracts

### Requirement: Cairn policy contracts resolve internal references
r[molten.project.cairn_policy_integrity.references] Cairn policy Nickel contracts MUST reject generated policy source when artifact dependencies, determinism surfaces, replay groups, replay cases, receipt contracts, or receipt schemas reference ids that are not declared in the same reviewed policy source.

#### Scenario: Policy with declared references exports
- GIVEN a Cairn policy whose artifact `requires` entries, replay surface refs, receipt schema commands, and receipt contracts all point to declared ids or commands
- WHEN the policy is evaluated through Nickel
- THEN the export succeeds and generated policy JSON preserves those reviewed relationships

#### Scenario: Policy with stale reference fails
- GIVEN a Cairn policy whose artifact dependency, replay case, replay group, receipt command, or determinism surface points to an unknown id
- WHEN the policy is evaluated through Nickel
- THEN export fails before stale policy JSON can be generated

### Requirement: Cairn policy ids and markers are unique
r[molten.project.cairn_policy_integrity.uniqueness] Cairn policy Nickel contracts MUST reject duplicate artifact ids, duplicate marker ids, duplicate marker tokens, duplicate replay case ids, duplicate replay group ids, and ambiguous receipt schema command entries.

#### Scenario: Distinct policy ids export
- GIVEN a Cairn policy with distinct artifact, marker, replay, and receipt schema identities
- WHEN the policy is evaluated through Nickel
- THEN export succeeds without introducing ambiguous policy lookup behavior

#### Scenario: Duplicate policy identity fails
- GIVEN a Cairn policy with a repeated marker token, artifact id, replay case id, replay group id, or receipt schema command
- WHEN the policy is evaluated through Nickel
- THEN export fails before ambiguous validation behavior can enter generated policy evidence
