# Upgrade Sessions Delta: Receipt-Backed Rails

## ADDED Requirements

### Requirement: Upgrade sessions are structured artifacts
r[molten.upgrades.structured_session_artifacts] Molten MUST define structured upgrade session artifacts for alias moves, artifact replacements, schema migrations, protocol drains, policy updates, handler-profile changes, transcript rewrites, and cleanup that bind affected refs, task graph, required evidence, rollback strategy, and non-claim boundary checks.

#### Scenario: Schema migration session names affected refs
- GIVEN a session plans a schema migration
- WHEN Molten validates the session artifact
- THEN it binds source schema refs, target schema refs, affected storage/artifact refs, task graph, required migration evidence, and rollback refs.

#### Scenario: Name-only session denies
- GIVEN a session names affected artifacts only by mutable display name
- WHEN Molten validates the plan for normative cutover
- THEN it denies until exact refs or admitted resolution receipts are bound.

### Requirement: Task state is receipt-backed metadata
r[molten.upgrades.receipt_backed_task_state] Molten MUST keep canonical upgrade plans immutable and record mutable task progress as receipts that point to the plan artifact and task ids.

#### Scenario: Task completion preserves plan hash
- GIVEN an upgrade plan has canonical ref P
- WHEN a task is marked complete with a receipt
- THEN P remains unchanged
- AND the task receipt binds P and the completed task id.

#### Scenario: Checkbox without receipt denies readiness
- GIVEN a task appears checked in rendered markdown but has no matching task receipt
- WHEN Molten evaluates cutover readiness
- THEN the task is not accepted as complete for normative gates.

### Requirement: Cutover gates bind subsystem evidence
r[molten.upgrades.cutover_gate_binding] Molten MUST require impact query, compatibility, migration, protocol-session, replay, policy, capability, resource, provenance, source-gate, and retention evidence appropriate to the affected refs before cutover or cleanup side effects.

#### Scenario: Protocol cutover requires terminal session evidence
- GIVEN an upgrade affects a protocol artifact
- WHEN cutover readiness is evaluated
- THEN Molten requires a passing protocol-session gate receipt with terminal state refs for the old protocol.

#### Scenario: Cleanup denies without retention evidence
- GIVEN an upgrade cleanup would delete or tombstone old artifacts
- WHEN readiness lacks retention and dependency impact evidence
- THEN Molten denies cleanup before destructive side effects.

### Requirement: Upgrade sessions do not replace source-control or build workflows
r[molten.upgrades.no_source_control_replacement] Molten MUST document that upgrade sessions are prior-art-inspired coordination evidence only and do not replace Git, Cargo, Nix, Cairn changes, repo-local tests, human review, or release gates.

#### Scenario: Session links external review evidence
- GIVEN an upgrade requires source changes
- WHEN the session records readiness
- THEN it may link source-control, build, test, Cairn, or review evidence refs
- AND it does not claim to replace those workflows.

#### Scenario: UCM compatibility claim denies
- GIVEN metadata claims Molten upgrade sessions are compatible with UCM patch/update semantics
- WHEN validation checks non-claim boundaries
- THEN it denies the compatibility claim.

### Requirement: Upgrade session validation covers positive and negative paths
r[molten.upgrades.session_validation] Molten MUST include positive and negative fixtures for gated cutover, stale impact evidence, missing protocol drain, incomplete migration, failed replay, unauthorized alias update, and destructive cleanup denial.

#### Scenario: Fully gated cutover fixture passes
- GIVEN a session has exact refs and all required subsystem receipts
- WHEN validation runs
- THEN Molten emits passing cutover readiness evidence.

#### Scenario: Failed replay fixture denies
- GIVEN a session affects transcripts or deterministic behavior and replay evidence fails
- WHEN cutover readiness is evaluated
- THEN Molten denies cutover
- AND diagnostics bind the failed replay receipt.