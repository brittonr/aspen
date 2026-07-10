# Evidence Gates Specification

## Purpose

Define Aspen/Molten ast-grep structural audits for runtime authority boundaries and evidence-gate receipts.

## Requirements

### Requirement: Runtime-authority audit profiles
r[aspen.ast_grep_runtime_authority_audits.profile] Aspen MUST define ast-grep audit profiles for syntax-level checks across runtime authority, node control, effect handler, plugin host, sealed-repro, transport, policy, and evidence-gate surfaces.

#### Scenario: Authority audit profile is declared
- GIVEN Aspen declares an ast-grep runtime-authority profile
- WHEN evidence-gate tooling enumerates checks
- THEN the profile MUST be identified as structural candidate evidence with explicit non-claims.

### Requirement: Inventory-first authority scans
r[aspen.ast_grep_runtime_authority_audits.inventory] Aspen SHOULD start ast-grep authority rules as inventory-only for ambient filesystem, process, network, clock, random, credential, plugin-loading, unsafe, panic, and direct authority-bypass source shapes.

#### Scenario: Network call candidate is found
- GIVEN ast-grep finds a direct network call in a runtime boundary surface
- WHEN the scan receipt is emitted
- THEN Aspen SHOULD report a candidate authority finding and MUST NOT claim distributed unsafety solely from the match.

### Requirement: Receipt identity
r[aspen.ast_grep_runtime_authority_audits.identity] Aspen MUST bind ast-grep version, rule bundle BLAKE3 identity, scan scope, runtime or evidence-gate run identity, findings summary, and non-claim labels into receipts.

#### Scenario: Evidence gate references changed rule bundle
- GIVEN an evidence-gate receipt references an ast-grep rule bundle identity
- WHEN rule files change
- THEN Aspen MUST require a fresh scan receipt before using the finding summary.

### Requirement: Positive and negative fixtures
r[aspen.ast_grep_runtime_authority_audits.fixtures] Aspen MUST require positive and negative fixtures before an ast-grep runtime-authority rule can become warning or blocking.

#### Scenario: Allowed shell effect is false-positive
- GIVEN an ast-grep rule flags an allowed shell-owned effect
- WHEN fixture validation runs
- THEN negative coverage MUST prevent the rule from becoming blocking until scope or constraints are corrected.

### Requirement: Evidence-gate non-claims
r[aspen.ast_grep_runtime_authority_audits.evidence_gates] Aspen MUST report ast-grep findings through evidence-gate receipts without claiming runtime authority admission, replay correctness, sealed-repro correctness, UCAN authorization, distributed safety, or release readiness.

#### Scenario: Clean scan accompanies replay evidence
- GIVEN a replay evidence bundle includes a clean ast-grep scan summary
- WHEN the bundle is reviewed
- THEN Aspen MUST treat the scan as structural hygiene evidence only and MUST NOT use it as replay proof.

### Requirement: Audit validation evidence
r[aspen.ast_grep_runtime_authority_audits.validation] Aspen MUST validate ast-grep runtime-authority audits with rule tests, authority-boundary fixture scans, Cairn gates, and focused Aspen/Molten validation rails.

#### Scenario: Authority audit changes
- GIVEN an ast-grep runtime-authority audit profile changes
- WHEN validation evidence is assembled
- THEN the evidence MUST include positive fixtures, negative fixtures, scan receipts, lifecycle gates, and focused validation output.
