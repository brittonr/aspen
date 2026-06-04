## ADDED Requirements

### Requirement: TigerStyle remediation targets strict Octet passability
r[molten.octet_tigerstyle_remediation.spec.strict_passability] Octet/TigerStyle remediation work MUST reduce or review findings in a measured path toward strict fail-close, prioritizing critical evidence-bearing surfaces before low-risk style cleanup.

#### Scenario: Remediation records before and after evidence
- GIVEN a remediation slice changes a critical source path
- WHEN the slice is validated
- THEN it records before/after Octet finding counts, object corpus or fingerprint refs for changed critical paths, focused test results, and any remaining review/quarantine refs

#### Scenario: Style-only cleanup does not hide critical debt
- GIVEN critical findings remain on admission, harness, job execution, node startup, ledger/evidence, adapter, or redaction paths
- WHEN a remediation report claims strict readiness based only on import/style churn
- THEN the source-gate review rejects the claim until critical findings are removed or reviewed

### Requirement: Critical evidence paths avoid panic, unwrap, and ambient time
r[molten.octet_tigerstyle_remediation.spec.critical_caveats] Critical evidence-bearing paths MUST NOT rely on panic/unwrap/expect, ambient wall-clock or entropy, unchecked narrowing/division, unbounded loops, or sentinel fallbacks unless an explicit review receipt binds the exact finding and strict profile.

#### Scenario: Panic in gate validator denies strict readiness
- GIVEN a gate validator or report parser contains an unreviewed panic/unwrap finding
- WHEN strict Octet readiness is evaluated
- THEN the readiness gate denies and points to the finding key

#### Scenario: Ambient time is isolated behind receipt
- GIVEN a node startup or adapter shell needs wall-clock capture for diagnostics
- WHEN the path is evidence-bearing
- THEN the ambient observation is isolated in an explicit shell receipt and not used by the deterministic core decision

### Requirement: Evidence builders use validated input structs
r[molten.octet_tigerstyle_remediation.spec.input_structs] Receipt and canonical value builders on critical paths SHOULD use validated input structs or typed context records instead of long positional argument lists, and MUST fail closed on missing, duplicated, stale, or mismatched refs.

#### Scenario: Startup receipt builder validates fields before render
- GIVEN a node startup receipt input with adapter receipts out of deterministic order or missing profile refs
- WHEN the receipt is built
- THEN validation fails before rendering a pass receipt

#### Scenario: Job execution receipt avoids field-order bugs
- GIVEN a job execution receipt input with mismatched admission, sync, or closure refs
- WHEN the builder validates the input struct
- THEN it emits a denial instead of constructing a misleading canonical receipt

### Requirement: Critical collections and loops are bounded
r[molten.octet_tigerstyle_remediation.spec.resource_bounds] Critical runtime, harness, job, node, adapter, catalog, and report paths MUST declare deterministic bounds or prior artifact-derived limits for loops, queues, vectors, diagnostics, closure traversals, receipt lists, and trace builders.

#### Scenario: Job closure traversal has explicit limit
- GIVEN a remote job admission or execution recomputes artifact closure
- WHEN the closure exceeds the configured bound
- THEN the operation emits a canonical denial receipt
- AND no partial execution or hidden side effect occurs

#### Scenario: Diagnostics list is bounded
- GIVEN a failing source gate produces many diagnostics
- WHEN diagnostics exceed the profile limit
- THEN the receipt truncates or chunks diagnostics according to explicit policy and records the truncation check

### Requirement: Public evidence boundaries use typed refs
r[molten.octet_tigerstyle_remediation.spec.typed_refs] Public or cross-boundary APIs for artifacts, schemas, policies, receipts, capabilities, secrets, effect logs, profiles, node ids, peer ids, job refs, and state refs MUST parse raw CLI strings at the edge into typed/validated refs before entering core evidence logic.

#### Scenario: CLI short id expands before core call
- GIVEN a CLI command receives a short artifact id
- WHEN it calls a job, node, storage, or source-gate core function
- THEN the short id has already been expanded to a full typed ref or the command denies

#### Scenario: Raw capability string cannot cross admission boundary
- GIVEN a public admission helper accepts capability evidence
- WHEN a raw string lacks validated capability-ref shape and authority context binding
- THEN the helper denies before evaluating policy

### Requirement: Module splits preserve canonical identity
r[molten.octet_tigerstyle_remediation.spec.refactor_identity] Refactors performed for Octet/TigerStyle remediation MUST preserve existing canonical Preserves identity, receipt schemas, CLI output contracts, and replay behavior unless an explicit versioned schema change and migration receipt is added.

#### Scenario: Job DAG split preserves refs
- GIVEN `src/job_dag.rs` is split into submodules
- WHEN existing job DAG fixtures, execution requests, and receipts are recomputed
- THEN their canonical refs remain unchanged unless a versioned schema migration is recorded

#### Scenario: CLI split preserves failure artifacts
- GIVEN `src/main.rs` dispatch is split into command modules
- WHEN a command configured with `--failure-out` fails
- THEN it still writes the same canonical failure artifact shape and exit behavior
