## ADDED Requirements

### Requirement: End-to-end repository lifecycle test

An integration test SHALL exercise the full repository lifecycle: create repo, create commit with tree and blob, push to main, verify head.

#### Scenario: Repo create-commit-push roundtrip

- **WHEN** a test creates a repo, stores a blob, builds a tree, commits, and sets `heads/main`
- **THEN** `get_head` SHALL return the commit hash
- AND the commit's tree SHALL contain the stored blob

### Requirement: End-to-end issue lifecycle test

An integration test SHALL exercise the full issue lifecycle: create, comment, label, close, reopen, resolve.

#### Scenario: Issue CRUD roundtrip

- **WHEN** a test creates an issue, adds 2 comments, adds a label, closes it, then reopens it
- **THEN** `resolve_issue` SHALL return state `Open` with 2 comments and 1 label

### Requirement: End-to-end patch lifecycle test

An integration test SHALL exercise the full patch lifecycle: create, approve, merge, verify ref advancement.

#### Scenario: Patch create-approve-merge roundtrip

- **WHEN** a test creates a repo with initial commit, creates a patch with a new commit, approves it, and merges it
- **THEN** `get_head` SHALL return the merge commit
- AND `resolve_patch` SHALL show state `Merged`

### Requirement: End-to-end discussion lifecycle test

An integration test SHALL exercise the full discussion lifecycle: create, reply, resolve thread, close.

#### Scenario: Discussion CRUD roundtrip

- **WHEN** a test creates a discussion, adds 3 replies (one threaded), resolves a thread, closes the discussion
- **THEN** `resolve_discussion` SHALL return state `Closed` with 3 replies and 1 resolved thread

### Requirement: Multi-node gossip propagation test

An integration test SHALL verify that gossip announcements propagate between ForgeNodes sharing a gossip instance.

#### Scenario: Ref update gossip

- **WHEN** node A pushes a ref update to repo `R`
- **THEN** node B's gossip handler SHALL receive the RefUpdate announcement within 5 seconds

### Requirement: Fork integration test

An integration test SHALL verify that forking creates a linked repo with shared refs.

#### Scenario: Fork and verify

- **WHEN** a test creates repo `upstream` with 3 commits on main, then forks it as `my-fork`
- **THEN** `my-fork` SHALL have the same `heads/main` as `upstream`
- AND `my-fork`'s identity SHALL have `fork_info` pointing to `upstream`

### Requirement: Property-based COB resolution idempotency

A property test SHALL verify that applying the same change sequence twice produces the same resolved state.

#### Scenario: Issue resolution idempotency

- **WHEN** a random sequence of issue operations is generated and applied, then the same sequence is applied to a fresh Issue
- **THEN** both resolved states SHALL be identical

#### Scenario: Discussion resolution idempotency

- **WHEN** a random sequence of discussion operations is generated and applied twice
- **THEN** both resolved states SHALL be identical

### Requirement: Property-based COB convergence

A property test SHALL verify that independent changes applied in different orders produce the same final state for collection fields (labels, comments, reactions).

#### Scenario: Label convergence

- **WHEN** two independent AddLabel operations are applied in order `[A, B]` and `[B, A]`
- **THEN** both orderings SHALL produce the same label set

#### Scenario: Comment convergence

- **WHEN** two independent Comment operations are applied in both orderings
- **THEN** both orderings SHALL produce the same set of comment bodies (order may differ)

### Requirement: Property-based CobChange DAG well-formedness

A property test SHALL verify that generated CobChange DAGs maintain structural invariants.

#### Scenario: DAG acyclicity

- **WHEN** a random DAG of CobChanges is generated
- **THEN** topological sort SHALL succeed (no cycles)
- AND every parent reference SHALL point to a change that exists in the DAG

#### Scenario: Single root

- **WHEN** a random DAG of CobChanges is generated
- **THEN** exactly one change SHALL have empty parents (the root)
