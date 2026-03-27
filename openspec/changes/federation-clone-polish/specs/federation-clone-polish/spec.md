## ADDED Requirements

### Requirement: Incremental federation sync deduplicates objects

When the client sends `have_hashes` from a previous sync, the origin SHALL skip objects already known to the client.

#### Scenario: Second sync after initial clone

- **WHEN** Bob syncs after an initial clone (sending content hashes from the first sync as `have_hashes`), and Alice has not added new commits
- **THEN** the response contains 0 git objects (all skipped)

#### Scenario: Incremental sync after new commit

- **WHEN** Alice pushes a second commit and Bob re-syncs with `have_hashes` from the first sync
- **THEN** the response contains only the new objects (second commit's blob, tree, commit) not the first commit's objects

### Requirement: Multi-branch ref translation

When a federated repo has multiple branches pointing to different commits, each mirror ref SHALL point to the correct locally imported BLAKE3 hash.

#### Scenario: Two branches, two commits

- **WHEN** Alice has `heads/main` and `heads/dev` pointing to different commits
- **THEN** Bob's mirror refs each point to the correct locally imported commit BLAKE3

### Requirement: Clean production logging

Diagnostic info-level logs added during debugging SHALL be removed or downgraded to debug/trace.

#### Scenario: Normal federation sync

- **WHEN** a federation git clone or fetch runs without errors
- **THEN** only summary-level info logs appear (sync complete, refs/objects counts), not per-object KV read/write logs
