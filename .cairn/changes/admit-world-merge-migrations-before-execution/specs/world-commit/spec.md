# World Merge Migration Admission Delta

## ADDED Requirements

### Requirement: Original migration facts receive admission before execution

r[molten.audit_f06.admission] Molten MUST admit the original source schema, target schema, migration profile, binding identity, and input bounds before it invokes a migration adapter.

#### Scenario: Exact migration receives admission

- GIVEN available bounded source bytes and a current admitted binding for their exact source and target schemas
- WHEN merge preparation requests conversion
- THEN the core MUST return an explicit admitted conversion plan before the shell invokes the adapter.

#### Scenario: Binding lacks admission

- GIVEN a source-to-target binding with `admitted: false`
- WHEN merge preparation runs
- THEN it MUST deny conversion before any migration, handler, or publication port call.

#### Scenario: Binding does not match the original source

- GIVEN a missing binding, malformed profile, unavailable source, or source-schema mismatch
- WHEN merge preparation runs
- THEN it MUST return a typed denial without replacing source schema metadata.

### Requirement: Materialization preserves original identity and schema evidence

r[molten.audit_f06.binding] Molten MUST bind each materialized result to its admitted conversion plan, original root and schema, target schema, migration identity, and bounded output bytes.

#### Scenario: Matching conversion result is available

- GIVEN a result that matches the admitted plan and output limits
- WHEN the core validates the result
- THEN it MUST retain the original conversion evidence alongside the target value.

#### Scenario: Target normalization hides an unadmitted source

- GIVEN equal target-schema metadata after preparation but no admitted conversion evidence for the original source
- WHEN merge admission evaluates the prepared values
- THEN it MUST reject the conversion rather than infer admission from schema equality.

### Requirement: Result rejection precedes publication

r[molten.audit_f06.publication] Molten MUST reject stale, substituted, malformed, oversized, or failed migration results before it publishes output roots or a merge commit.

#### Scenario: Valid materialization reaches publication

- GIVEN all conversion results pass validation and current merge authority permits publication
- WHEN the shell publishes the merge
- THEN it MUST use only the validated output values and preserve the declared causal parents.

#### Scenario: Adapter returns incompatible bytes

- GIVEN a conversion result with the wrong target binding or an exceeded output bound
- WHEN result validation runs
- THEN Molten MUST NOT publish a generated root or merge commit, and existing heads MUST remain unchanged.

### Requirement: Migration admission has scoped regression evidence

r[molten.audit_f06.validation] Molten MUST record positive and negative core and shell tests for admission order, result binding, and publication denial without treating a source review as execution evidence.

#### Scenario: Shell regression verifies call order

- GIVEN a recording adapter and an unadmitted migration binding
- WHEN the regression runs
- THEN it MUST observe zero migration calls and zero publication calls.

#### Scenario: Only direct core checks pass

- GIVEN no test exercises the preparation shell with an unadmitted binding
- WHEN completion evidence is assessed
- THEN the change MUST remain incomplete.
