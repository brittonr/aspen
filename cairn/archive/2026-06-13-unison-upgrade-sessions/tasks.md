## Phase 1: Upgrade plan model

- [x] [serial] r[molten.upgrades.session_model] Define upgrade session plan DTOs with session id, affected artifacts, metadata moves, impact set, tasks, policies, rollback rules, and evidence refs.
- [x] [serial] r[molten.upgrades.task_model] Define initial upgrade task kinds for install, name move, compatibility alias, deprecate, migrate, transcript rerun, cutover, rollback, and cleanup.
- [x] [serial] r[molten.upgrades.plan_hashing] Hash canonical upgrade plans as artifacts while keeping task status as receipt-backed metadata.
- [x] [parallel] r[molten.upgrades.no_ucm_clone] Document that Unison structured refactoring is prior art only and Molten does not adopt UCM or replace Git/Cargo/Nix workflows.

## Phase 2: Impact and admission

- [x] [serial] r[molten.upgrades.impact_analysis] Compute impacted artifacts, reverse dependencies, storage refs, protocol sessions, docs, and transcripts from registry indexes. (First local implementation scans the evidence ledger for reverse refs until registry indexes land.)
- [x] [serial] r[molten.upgrades.plan_admission] Gate upgrade session creation through Nickel/Basalt/Trellis policy and required capabilities. (Local pass evidence requires explicit policy/capability refs; full Nickel/Basalt/Trellis engines remain future integration.)
- [x] [serial] r[molten.upgrades.task_receipts] Emit and validate Cairn receipts for task admission, completion, denial, cutover, rollback, and cleanup.
- [x] [parallel] r[molten.upgrades.compatibility_window] Model compatibility windows where old and new artifacts remain valid under explicit policy.

## Phase 3: First workflows

- [x] [serial] r[molten.upgrades.name_move_workflow] Implement a minimal workflow that moves a name/alias from one artifact id to another with impact analysis and receipts.
- [x] [serial] r[molten.upgrades.transcript_gate] Require selected executable transcripts to pass under declared handler profiles before cutover. (First slice binds transcript/handler evidence refs before cutover; actual transcript execution can reuse harness/eval-cache later.)
- [x] [parallel] r[molten.upgrades.storage_migration_hook] Connect upgrade tasks to typed-storage migration recipe artifacts once typed storage exists.
- [x] [parallel] r[molten.upgrades.protocol_drain_hook] Connect upgrade tasks to protocol session drain/compatibility checks once choreography sessions are durable.

## Phase 4: Cleanup and tests

- [x] [serial] r[molten.upgrades.cleanup_safety] Deny artifact cleanup unless registry indexes show no active sessions, durable refs, receipts, policies, docs, or pins require it.
- [x] [serial] r[molten.upgrades.rollback_tests] Add tests for reversible metadata rollback and explicit denial of irreversible rollback claims.
- [x] [parallel] r[molten.upgrades.todo_cli] Add CLI inspection for upgrade session status and remaining task todos.
- [x] [parallel] r[molten.upgrades.property_tests] Add Hegel property tests for task ordering, impact-set monotonicity, compatibility-window invariants, and cleanup safety.
