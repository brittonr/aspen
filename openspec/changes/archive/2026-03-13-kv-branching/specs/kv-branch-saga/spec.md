## ADDED Requirements

### Requirement: Branch-backed saga step execution

`SagaExecutor` SHALL support a branch-backed execution mode where a saga step runs inside a `BranchOverlay`. All KV writes performed by the step SHALL be buffered in the branch. On step success, the branch commit and the saga journal entry SHALL be written in a single atomic `WriteCommand::Batch`.

#### Scenario: Step succeeds with branch commit

- **WHEN** a saga step runs inside a branch
- **AND** the step writes keys A and B to KV
- **AND** the step completes successfully
- **THEN** a single `WriteCommand::Batch` SHALL be issued containing Set(A), Set(B), and the saga journal entry marking the step complete
- **AND** all three writes SHALL be applied atomically

#### Scenario: Step fails with automatic rollback

- **WHEN** a saga step runs inside a branch
- **AND** the step writes key A to KV
- **AND** the step then fails with an error
- **THEN** the branch SHALL be dropped
- **AND** key A SHALL NOT appear in the base store
- **AND** no compensation function SHALL be needed for KV writes

#### Scenario: Node crash during branched step

- **WHEN** a saga step is running inside a branch
- **AND** the node crashes before the branch is committed
- **THEN** the branch's in-memory state SHALL be lost
- **AND** the saga journal SHALL show the step as "started" but not "completed"
- **AND** recovery SHALL replay the step from scratch with a new branch

### Requirement: Per-step branch opt-in

Branch-backed execution SHALL be opt-in per saga step. Steps with external side effects (API calls, deploys, emails) SHALL continue to use the existing compensation model. The saga definition SHALL allow marking steps as `branch_backed: true`.

#### Scenario: Mixed saga with branch and compensation steps

- **WHEN** a saga has step 1 (branch-backed, KV-only) and step 2 (compensation-backed, calls external API)
- **AND** step 2 fails
- **THEN** step 2 SHALL run its compensation function
- **AND** step 1's committed KV writes SHALL remain (they were already committed atomically)

#### Scenario: Branch-backed step skips compensation

- **WHEN** a branch-backed saga step fails
- **THEN** the saga executor SHALL NOT invoke a compensation function for that step
- **AND** the step's `requires_compensation` flag SHALL be treated as false for branched steps

### Requirement: Commit includes journal write

The branch commit for a saga step SHALL include the saga state persistence write in the same atomic batch. The journal write and the step's KV writes SHALL either both succeed or both fail.

#### Scenario: Atomic commit with journal

- **WHEN** a branch-backed step has dirty writes [Set("data/x", "1")] and the saga journal write is Set("__saga_state::abc", "{...}")
- **THEN** the commit batch SHALL contain both operations
- **AND** they SHALL be applied in a single Raft log entry
