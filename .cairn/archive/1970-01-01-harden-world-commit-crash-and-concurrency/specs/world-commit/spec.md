# Molten World Commit Specification Delta

## Purpose

Add bounded crash, restart, uncertainty, and concurrency conformance across every supported world mutation boundary.

## ADDED Requirements

### Requirement: Every world mutation has a registered failure contract

r[molten.world_faults.inventory] Molten MUST maintain a closed versioned inventory for capture, head, promotion, witness, outbox, replication, import, retention, and garbage-collection mutations. Each row MUST name its owner, operation identity, expected pre-state, effects, linearization point, durable record, uncertain window, reconciliation entry, and required cases.

#### Scenario: New mutation is not registered

- GIVEN product code adds a world-related durable mutation absent from the inventory
- WHEN conformance coverage runs
- THEN the rail MUST fail before claiming complete mutation coverage

### Requirement: Fault profiles use named semantic phases

r[molten.world_faults.profile] Fault profiles MUST use bounded named operation phases and explicit schedules. They MUST bind profile, adapter, limit, source revision, and operation identities with BLAKE3. Source-line locations and wall-clock timing MUST NOT establish semantic phase or winner selection.

#### Scenario: Lost response follows possible submit

- GIVEN a profile interrupts after possible external submission but before response
- WHEN the harness records the observation
- THEN the result MUST enter the owning uncertain-outcome path

#### Scenario: Profile contains an unexplained numeric threshold

- GIVEN a fault profile uses a numeric limit without a named typed field and contract
- WHEN profile validation runs
- THEN Molten MUST reject the profile

### Requirement: The shell injects faults without owning decisions

r[molten.world_faults.shell_boundary] The imperative harness MAY control interruption, restart, storage, and schedules. It MUST pass observations to the owning pure cores and MUST NOT synthesize success, compensation, conflict, or recovery decisions.

#### Scenario: Adapter reports a successful write without durable read-back

- GIVEN the owning contract requires read-back and the shell has no matching observation
- WHEN conformance comparison runs
- THEN the harness MUST NOT mark the operation complete

### Requirement: Concurrency uses explicit fenced schedules

r[molten.world_faults.concurrency] Concurrent tests MUST bind operation IDs, expected generations, pre-state identities, and declared interleaving points. Competing valid transitions MUST preserve compare-and-swap, conflict, authority, and effect-release rules.

#### Scenario: Two promotions target one generation

- GIVEN two admitted promotion plans name the same expected head and different successors
- WHEN the deterministic schedule interleaves final publication
- THEN at most one transition MAY publish at that generation
- AND the other MUST become stale, superseded, or conflicting without duplicate effect release

### Requirement: Restart recovery remains conservative

r[molten.world_faults.recovery] Recovery MUST classify durable observations as already-complete, safe-to-retry, superseded, conflict, uncertain, denied, corrupt, or manual-review. Missing, contradictory, or incomplete state MUST NOT become success or cleanup authority.

#### Scenario: Commit outcome is ambiguous after restart

- GIVEN an operation record exists but required publication read-back is missing
- WHEN recovery classification runs
- THEN Molten MUST return uncertain or manual-review according to the owning contract

### Requirement: Interruption cases cover every mutation window

r[molten.world_faults.interruption] Each registered mutation MUST have positive uninterrupted coverage and negative before-submit, after-possible-submit, after-durable-write, lost-response, restart, and recovery coverage where those phases apply.

#### Scenario: Required phase has no fixture

- GIVEN an inventory row declares a lost-response window without a matching fixture
- WHEN coverage validation runs
- THEN that row MUST remain incomplete

### Requirement: Rollback tests preserve witness boundaries

r[molten.world_faults.verification] Local-profile rollback tests MUST NOT claim detection when head and generation state roll back together. Strong rollback cases MUST use independent admitted witness state.

#### Scenario: Only local image is restored

- GIVEN the harness restores an older complete local image and no independent witness state exists
- WHEN rollback classification runs
- THEN the result MUST state that whole-store rollback detection is unproven

### Requirement: Fault receipts are bounded evidence

r[molten.world_faults.receipt] Conformance receipts MUST bind the inventory, profile, source revision, adapters, schedules, limits, cases, observations, decisions, and unsupported rows. They MUST NOT claim universal crash safety, physical power-loss coverage, storage correctness, or release eligibility.

#### Scenario: Focused matrix passes

- GIVEN every required bounded case passes for one reviewed cohort
- WHEN the final receipt is emitted
- THEN it MUST identify that cohort and preserve all physical-failure non-claims
