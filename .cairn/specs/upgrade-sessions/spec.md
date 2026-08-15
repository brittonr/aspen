# Upgrade Sessions Specification

## Purpose

Defines receipt-backed upgrade sessions for immutable Molten artifacts, metadata pointers, protocol sessions, typed storage migrations, transcripts, and cleanup.

## Requirements

### Requirement: System MUST Define upgrade session plan DTOs with session id, affected artifacts, metadata moves, impact set, tasks, policies, rollback rules, and evidence refs
r[molten.upgrades.session_model] The system MUST define canonical upgrade session plan DTOs with session id, affected artifacts, metadata moves, impact set, tasks, policies, rollback rules, and evidence refs.

### Requirement: System MUST Define initial upgrade task kinds for install, name move, compatibility alias, deprecate, migrate, transcript rerun, cutover, rollback, and cleanup
r[molten.upgrades.task_model] The system MUST define initial upgrade task kinds for install, name move, compatibility alias, deprecate, migrate, transcript rerun, cutover, rollback, and cleanup.

### Requirement: System MUST Hash canonical upgrade plans as artifacts while keeping task status as receipt-backed metadata
r[molten.upgrades.plan_hashing] The system MUST hash canonical upgrade plans as artifacts while keeping mutable task status as receipt-backed metadata.

### Requirement: System MUST Document that Unison and UCM are prior art only
r[molten.upgrades.no_ucm_clone] The system MUST document that Unison structured refactoring and UCM are non-normative prior art and MUST NOT replace Git, Cargo, or Nix workflows.

### Requirement: System MUST Compute impacted artifacts, durable refs, active protocol sessions, docs, and transcripts from registry indexes or bounded ledger scans
r[molten.upgrades.impact_analysis] The system MUST compute impacted artifacts, durable refs, active protocol sessions, docs, and transcripts from registry indexes when available or bounded ledger scans as fallback.

### Requirement: System MUST Gate upgrade session creation through policy and required capabilities
r[molten.upgrades.plan_admission] The system MUST gate upgrade session creation through explicit policy refs, required capability refs, and source-gate evidence before mutation.

### Requirement: System MUST Emit and validate receipts for task admission, completion, denial, cutover, rollback, and cleanup
r[molten.upgrades.task_receipts] The system MUST emit and validate receipts for upgrade task admission, completion, denial, cutover, rollback, and cleanup.

### Requirement: System MUST Model explicit compatibility windows where old and new artifacts remain valid under policy
r[molten.upgrades.compatibility_window] The system MUST model compatibility windows where old and new artifacts remain valid concurrently under explicit policy refs.

### Requirement: System MUST Implement a minimal workflow that moves a name or alias from one artifact id to another with impact analysis and receipts
r[molten.upgrades.name_move_workflow] The system MUST implement a minimal workflow that moves a name or alias from one artifact id to another with impact analysis and receipts.

### Requirement: System MUST Require selected executable transcript evidence before cutover
r[molten.upgrades.transcript_gate] The system MUST require selected executable transcript or transcript receipt evidence before cutover.

### Requirement: System MUST Connect upgrade tasks to typed-storage migration recipe artifacts
r[molten.upgrades.storage_migration_hook] The system MUST connect upgrade tasks to typed-storage migration recipe artifacts and migration receipts when storage schemas change.

### Requirement: System MUST Connect upgrade tasks to protocol session drain and compatibility checks
r[molten.upgrades.protocol_drain_hook] The system MUST connect `drain-sessions` upgrade tasks to passing `protocol-session-gate-receipt-v1` evidence for the affected protocol artifact, including terminal session-state refs, before admitting protocol cutover or related task completion.

#### Scenario: Protocol drain task rejects missing or mismatched gate evidence
r[molten.upgrades.protocol_drain_hook.gate]
- GIVEN a `drain-sessions` upgrade task with missing, denied, stale, or wrong-protocol protocol session gate evidence
- WHEN the task is executed
- THEN the task receipt decision is `deny` and diagnostics identify the missing, denied, terminal-state, or protocol-ref binding failure before cutover side effects.

### Requirement: System MUST Deny artifact cleanup unless registry indexes prove no active references require it
r[molten.upgrades.cleanup_safety] The system MUST deny artifact cleanup unless registry indexes or bounded ledger scans show no active sessions, durable refs, receipts, policies, docs, pins, or metadata pointers require the artifact.

### Requirement: System MUST Add tests for reversible metadata rollback and denial of irreversible rollback claims
r[molten.upgrades.rollback_tests] The system MUST add tests for reversible metadata rollback and explicit denial of irreversible rollback claims.

### Requirement: System MUST Add CLI inspection for upgrade session status and remaining task todos
r[molten.upgrades.todo_cli] The system MUST add CLI inspection for upgrade session status and remaining task todos.

### Requirement: System MUST Add Hegel property tests for task ordering, impact monotonicity, compatibility windows, and cleanup safety
r[molten.upgrades.property_tests] The system MUST add Hegel property tests for task ordering, impact-set monotonicity, compatibility-window invariants, and cleanup safety.

### Requirement: Upgrade drains require terminal protocol evidence
r[molten.upgrade_drain_state_proof.terminal_protocol_gate] Molten MUST prove that protocol-drain tasks complete only when a passing protocol-session gate binds the affected old protocol ref and at least one terminal session-state ref.

#### Scenario: Empty terminal states deny drain
- GIVEN an upgrade drain task with a protocol-session gate receipt that lists no terminal final states
- WHEN drain completion is evaluated
- THEN the upgrade receipt decision is `deny`
- AND cutover side effects are not emitted.

### Requirement: Upgrade drain protocol refs are exact
r[molten.upgrade_drain_state_proof.protocol_ref_binding] Molten MUST prove that protocol-drain evidence matches the task `from_ref` or explicit affected/compatibility refs and denies stale or wrong-protocol gates before mutation.

#### Scenario: Wrong protocol gate denies cutover
- GIVEN an upgrade task from protocol ref `old`
- WHEN the supplied lifecycle gate receipt binds protocol ref `other`
- THEN the drain decision is `deny`
- AND diagnostics identify the wrong protocol ref.

### Requirement: Upgrade denial preserves pre-cutover state
r[molten.upgrade_drain_state_proof.no_mutation_on_deny] Molten MUST prove that missing, denied, stale, malformed, or wrong-protocol drain evidence leaves registry, routing, compatibility, and artifact refs unchanged.

#### Scenario: Stale compatibility ref leaves routing unchanged
- GIVEN a drain task with stale compatibility evidence
- WHEN upgrade cutover admission runs
- THEN the decision is `deny`
- AND before/after routing or registry refs are identical.

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
