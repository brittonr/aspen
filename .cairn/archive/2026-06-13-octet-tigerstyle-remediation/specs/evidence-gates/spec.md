## ADDED Requirements

### Requirement: Octet remediation metrics are canonical evidence
r[molten.octet_tigerstyle_remediation.baseline_metrics] Molten MUST capture Octet/TigerStyle remediation metrics as canonical evidence that binds workspace, lib-only, and focused critical-path status refs, summary refs, object-corpus refs, finding counts, warning counts, error counts, autofixable counts, and plan refs.

#### Scenario: Remediation plan binds current counts
- GIVEN workspace and lib-only Octet artifacts
- WHEN Molten builds the remediation plan
- THEN the plan records status, summary, object-corpus refs, counts, diagnostics, and checks for those scopes.

### Requirement: Critical source surfaces are inventoried
r[molten.octet_tigerstyle_remediation.critical_surface_inventory] Molten MUST inventory critical source surfaces relevant to Octet/TigerStyle remediation, including source-gate/admission, harness/gates, node runtime startup, job execution, ledger/evidence, adapter boundaries, redaction/export, and CLI artifact-output paths.

#### Scenario: Critical surface lists source files
- GIVEN the remediation plan is rendered
- WHEN an operator inspects critical surfaces
- THEN each surface lists source files, warning counts, critical counts, and rationale.

### Requirement: Remediation priority is explicit
r[molten.octet_tigerstyle_remediation.priority_order] Molten MUST prioritize Octet/TigerStyle burn-down work as critical deny classes first, resource bounds second, high-arity and long functions third, file/module splits fourth, and style/autofix cleanup last.

#### Scenario: Critical finding outranks style churn
- GIVEN both a critical source-gate finding and a style-only import finding exist
- WHEN remediation work is scheduled
- THEN the critical source-gate finding is scheduled first unless a review receipt explains the exception.

### Requirement: Hidden suppressions are forbidden
r[molten.octet_tigerstyle_remediation.no_suppression_policy] Molten MUST NOT treat hidden suppressions as remediation. Every retained active warning MUST have scheduled remediation, an explicit reviewed quarantine receipt, or a documented configuration caveat that strict consumers can distinguish from source-remediated zero.

#### Scenario: Disabled lint remains a caveat
- GIVEN an Octet lint family is disabled in configuration
- WHEN the remediation plan is inspected
- THEN the plan records the disabled family as a caveat or future burn-down item rather than hidden clean evidence.

### Requirement: Panic and unwrap caveats are removed or reviewed
r[molten.octet_tigerstyle_remediation.no_panic_unwrap] Molten MUST remove, deny, or review `panic`, `unwrap`, and `expect` findings on critical evidence-bearing paths before those paths can satisfy strict source-gate evidence.

#### Scenario: Critical unwrap requires review
- GIVEN a critical path contains an `unwrap` finding
- WHEN strict or quarantine source-gate evidence is evaluated
- THEN the finding denies unless an exact review manifest covers it temporarily for the profile.

### Requirement: Ambient clock caveats are isolated
r[molten.octet_tigerstyle_remediation.no_ambient_clock] Molten MUST remove ambient wall-clock/time findings from deterministic evidence paths or isolate them behind explicit shell receipts and source-gate review evidence.

#### Scenario: Clock use in deterministic core denies
- GIVEN Octet reports ambient clock use in a deterministic core path
- WHEN strict source-gate evidence is evaluated
- THEN the gate denies until the clock use is removed or isolated behind explicit receipt evidence.

### Requirement: Unbounded loops are bounded or reviewed
r[molten.octet_tigerstyle_remediation.no_unbounded_loops] Molten MUST add explicit limits, yield/checkpoints, or review receipts for unbounded loop and recursion findings in runtime, harness, job, adapter, and report paths before strict source-gate acceptance.

#### Scenario: Unbounded report loop denies
- GIVEN a report renderer accumulates unbounded data-dependent output
- WHEN Octet evaluates the critical surface
- THEN strict source-gate evidence denies unless a deterministic budget or review receipt covers the path.

### Requirement: Sentinel fallbacks are replaced by typed denial paths
r[molten.octet_tigerstyle_remediation.no_sentinel_fallbacks] Source-gate, admission, and evidence paths MUST avoid sentinel fallback refs or strings where typed option/result handling and canonical denial receipts are required.

#### Scenario: Missing ref becomes explicit denial
- GIVEN a required source-gate ref is absent
- WHEN startup, job admission, or upgrade planning validates evidence
- THEN Molten emits a deny receipt instead of substituting a synthetic passing sentinel.

### Requirement: Collections on evidence paths are bounded
r[molten.octet_tigerstyle_remediation.collection_bounds] Evidence-bearing runtime, job, node, harness, catalog, adapter, source-gate, and report paths MUST use deterministic bounds, validated prior limits, or explicit resource accounting for data-dependent collection growth.

#### Scenario: Finding index has a maximum
- GIVEN Octet structured findings are parsed
- WHEN the finding index is loaded
- THEN Molten enforces a maximum entry count before inserting into collections.

### Requirement: Builder input structs replace high-arity evidence helpers
r[molten.octet_tigerstyle_remediation.builder_input_structs] Molten SHOULD replace high-arity receipt/value builders on critical evidence paths with named input structs that validate typed refs and invariants before rendering canonical Preserves values. Remaining high-arity helpers MUST remain visible in the remediation plan or future burn-down work rather than hidden as clean evidence.

#### Scenario: Receipt builder uses named inputs
- GIVEN a critical receipt helper grows many fields
- WHEN it is remediated
- THEN a named input struct carries the fields and validation before canonical rendering.

### Requirement: Public evidence boundaries validate refs
r[molten.octet_tigerstyle_remediation.typed_ref_boundaries] Public evidence boundaries SHOULD replace raw strings and generic hashes with typed ref/id/profile structs or parsing functions that fail closed. Remaining raw-string boundaries MUST be limited to CLI/config parsing edges or documented future burn-down items.

#### Scenario: CLI string is parsed before runtime use
- GIVEN a CLI command accepts a source-gate receipt ref string
- WHEN runtime or admission logic consumes it
- THEN the value is parsed, validated, or denied before it is treated as evidence.

### Requirement: Remediation adds assertion coverage
r[molten.octet_tigerstyle_remediation.assertion_density] Remediation slices SHOULD add positive and negative assertions around pure helpers, denial paths, source-gate validators, collection bounds, and receipt binding behavior introduced by the cleanup.

#### Scenario: Denial helper has negative assertion
- GIVEN a source-gate validation helper denies stale evidence
- WHEN tests run
- THEN they assert both the deny decision and a diagnostic that identifies the stale evidence.

### Requirement: CLI shell split remains tracked
r[molten.octet_tigerstyle_remediation.cli_shell_split] Molten SHOULD split large CLI imperative-shell surfaces into smaller dispatch modules and pure command input conversion helpers over time. Until that source-remediated split is complete, the remediation evidence MUST document the caveat and MUST NOT claim that disabled file/function-size lint families represent source-remediated zero.

#### Scenario: CLI split is documented as future work
- GIVEN strict source-gate evidence is configuration-clean because file/function-size lint families are disabled
- WHEN the remediation plan is inspected
- THEN CLI/module split work remains listed as a burn-down item or caveat.

### Requirement: Job DAG split remains tracked
r[molten.octet_tigerstyle_remediation.job_dag_split] Molten SHOULD split large job DAG surfaces into DTO, parse, sync, admission, execution, memo/cache, and test-support modules without changing canonical refs. Until complete, current Octet evidence MUST distinguish configuration-clean status from source-remediated zero.

#### Scenario: Job DAG split preserves canonical refs
- GIVEN job DAG module splitting is performed in a future slice
- WHEN validation runs
- THEN canonical job refs, receipts, and replay outputs remain stable unless intentionally versioned.

### Requirement: Node runtime shape remains tracked
r[molten.octet_tigerstyle_remediation.node_runtime_shape] Molten SHOULD keep node runtime startup code shaped around typed inputs, bounded adapter lists, deterministic duplicate-free ordering, short receipt helpers, and deny receipts for failed startup. Remaining shape debt MUST stay visible in remediation evidence.

#### Scenario: Startup denial remains receipt-backed
- GIVEN source-gate evidence is missing or denied
- WHEN node startup evaluates configuration
- THEN startup emits a canonical deny receipt and starts no production adapters.

### Requirement: Object corpus evidence is refreshed after remediation
r[molten.octet_tigerstyle_remediation.object_corpus_refresh] Molten MUST refresh object-corpus/fingerprint evidence for changed critical paths before claiming strict source-gate pass evidence for those paths.

#### Scenario: Changed source path refreshes corpus
- GIVEN a critical source path changes during remediation
- WHEN source-gate evidence is produced
- THEN the object-corpus and fingerprint refs reflect the changed source scope.

### Requirement: Focused Octet runs are recorded
r[molten.octet_tigerstyle_remediation.focused_octet_runs] Remediation slices SHOULD re-run focused Octet checks after changes and record before/after finding deltas, even when the current result is configuration-clean.

#### Scenario: Focused run records zero findings
- GIVEN a focused critical-path Octet run reports zero findings
- WHEN the remediation plan is generated
- THEN it records the status and count refs for that focused evidence.

### Requirement: Strict profile dry-runs drive burn-down
r[molten.octet_tigerstyle_remediation.strict_profile_dry_run] Molten SHOULD run strict Octet gate dry-runs until warning-only status is eliminated or only reviewed noncritical debt remains under explicit quarantine. Configuration-clean strict passes MUST be labeled with disabled-lint caveats when applicable.

#### Scenario: Strict dry-run rejects warning-only
- GIVEN a strict profile dry-run sees warning-only status
- WHEN the gate evaluates it
- THEN it denies and records warning counts for the next burn-down slice.

### Requirement: Remediation must preserve canonical behavior
r[molten.octet_tigerstyle_remediation.no_regression_tests] Remediation SHOULD include tests or validation proving canonical refs, report receipts, job execution outputs, source-gate receipts, and node startup evidence remain stable except where intentionally versioned.

#### Scenario: Source gate ref remains deterministic
- GIVEN the same Octet artifacts and policy
- WHEN gate evaluation runs after a remediation-only refactor
- THEN the canonical gate receipt is stable unless an input artifact or version changed.

### Requirement: Cairn task drain follows source evidence
r[molten.octet_tigerstyle_remediation.cairn_task_drain] Molten MUST check off Octet fail-close, quarantine, and TigerStyle remediation tasks only when the corresponding code, documentation, caveat, strict gate receipt, or future-work evidence is present and validated by Cairn gates.

#### Scenario: Deferred split is not claimed as finished source cleanup
- GIVEN a module split remains future work
- WHEN the Cairn task package is archived
- THEN the accepted spec records the future-work caveat instead of claiming source-remediated zero.
