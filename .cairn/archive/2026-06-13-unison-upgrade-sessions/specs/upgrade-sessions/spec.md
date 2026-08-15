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
