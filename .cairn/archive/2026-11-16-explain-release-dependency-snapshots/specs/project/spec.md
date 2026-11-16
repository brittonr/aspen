# Release Dependency Explanation Delta

## ADDED Requirements

### Requirement: Release dependency explanations have explicit outcomes

r[molten.project.reproducible_dependencies.explanation_outcomes] Molten MUST evaluate release-dependency explanations for `LockDrift` and `MissingArchiveReceipt` freshly from one immutable `ReleaseDependencyInput` snapshot in pure core logic, and MUST report an unavailable or incomplete observation collection, unsupported input, or exhausted fact, step, or depth budget as an explicit refusal without a report or release pass. A completed evaluation MUST mirror the validator's decision for the same snapshot and MUST NOT change admission, canonical report material, or redaction.

#### Scenario: Incomplete collection refuses

- GIVEN an otherwise valid snapshot whose archive observation collection is declared incomplete
- WHEN explanations are evaluated
- THEN the outcome MUST be incomplete, carry no report, and MUST NOT admit the release

#### Scenario: Budget or unsupported input refuses

- GIVEN a snapshot whose rows exceed the validator bounds, limits outside their caps, or a fact, step, or depth budget smaller than the evaluation needs
- WHEN explanations are evaluated
- THEN the outcome MUST be an unsupported or budget-exhausted refusal and MUST NOT admit the release

#### Scenario: Output bound truncates explicitly

- GIVEN a completed evaluation whose rendered explanations exceed the output bound
- WHEN the report is built
- THEN the outcome MUST be truncated and MUST count the omitted explanations

#### Scenario: Replaced snapshot keeps no stale conclusion

- GIVEN explanations for a snapshot that is then corrected or loses an observation row
- WHEN the replacement is evaluated
- THEN the new report MUST be derived only from the replacement, and the earlier explanations MUST fail verification against it

### Requirement: Release dependency explanation references are snapshot-bound

r[molten.project.reproducible_dependencies.explanation_references] Every release-dependency explanation MUST name its rule and identify each supporting fact by relation, row occurrence, field, and value, or by an absent key in a relation declared completed. The report MUST bind the BLAKE3 identity of the exact snapshot, including row order and duplicates, the rule revision, the declared collections, and the limits. Verification MUST reject stale snapshots, unknown occurrences, changed values, rows of another subject, non-effective duplicate observations, absence over an uncompleted or contradicting relation, and support that does not satisfy the rule.

#### Scenario: Missing observation, missing field, and archive absence stay distinct

- GIVEN a completed snapshot with a missing dependency observation, a present observation lacking a lock field, an absent archive observation, and an observation reporting the archive absent
- WHEN explanations are evaluated
- THEN the missing dependency observation MUST yield no lock-drift explanation, the missing field MUST be supported as `missing`, and the two archive cases MUST use different rules

#### Scenario: Two packages are never joined

- GIVEN two packages whose observations carry each other's lock values
- WHEN explanations are evaluated and verified
- THEN every reference MUST name a row of the explained package, and a reference to the other package's row MUST be rejected

### Requirement: Release dependency explanation approach is compared before extension

r[molten.project.reproducible_dependencies.explanation_comparison] Molten MUST compare the structured Rust explanation path with a minimal relational pilot over one fixture corpus covering valid input, lock source and revision drift, absent observations, missing archives, duplicate observations, and crossed packages, and MUST record explanation agreement, projection and evaluation cost, and implementation complexity before adopting a rule engine or adding a rule family.

#### Scenario: Both paths agree with the validator

- GIVEN every corpus snapshot
- WHEN both paths explain it
- THEN they MUST produce identical explanations whose `(code, subject)` rows equal the validator's selected rows, and the pilot's references MUST pass independent verification
