## ADDED Requirements

### Requirement: Commit-to-commit diff via RPC

The system SHALL expose a `ForgeDiffCommits` RPC that accepts two commit hashes and returns structured diff entries with optional unified diff text.

#### Scenario: Diff two commits

- **GIVEN** repo `R` with commits `C1` and `C2` where `C2` modifies `src/main.rs`
- **WHEN** `ForgeDiffCommits { repo_id: R, old_commit: C1, new_commit: C2, include_content: false }` is sent
- **THEN** the response SHALL contain a `ForgeDiffResult` with one entry of kind `Modified` at path `src/main.rs`
- **AND** `unified_diff` SHALL be `None`

#### Scenario: Diff with unified text

- **GIVEN** repo `R` with commits `C1` and `C2` where `C2` modifies `config.toml`
- **WHEN** `ForgeDiffCommits { ..., include_content: true }` is sent
- **THEN** the response SHALL contain `unified_diff` with standard unified diff text
- **AND** the text SHALL include `--- a/config.toml` and `+++ b/config.toml` headers

#### Scenario: Unknown commit hash

- **GIVEN** commit hash `DEADBEEF` does not exist
- **WHEN** `ForgeDiffCommits { ..., old_commit: DEADBEEF }` is sent
- **THEN** the response SHALL be an error indicating the object was not found

### Requirement: Ref-to-ref diff via RPC

The system SHALL expose a `ForgeDiffRefs` RPC that resolves ref names to commits and delegates to commit diff.

#### Scenario: Diff two branches

- **GIVEN** repo `R` with branches `heads/main` and `heads/feature`
- **WHEN** `ForgeDiffRefs { repo_id: R, old_ref: "heads/main", new_ref: "heads/feature", include_content: true }` is sent
- **THEN** the response SHALL be equivalent to diffing the commits those refs point to

#### Scenario: Unknown ref

- **GIVEN** ref `heads/nonexistent` does not exist in repo `R`
- **WHEN** `ForgeDiffRefs { ..., old_ref: "heads/nonexistent" }` is sent
- **THEN** the response SHALL be an error indicating the ref was not found

### Requirement: Rename detection

The system SHALL detect file renames by matching content hashes between `Removed` and `Added` entries in a diff result.

#### Scenario: File renamed

- **GIVEN** commit `C1` has `old_name.rs` and commit `C2` has `new_name.rs` with identical content
- **WHEN** diff is computed between `C1` and `C2`
- **THEN** the result SHALL contain one entry of kind `Renamed` with `old_path: "old_name.rs"` and `path: "new_name.rs"`

#### Scenario: File renamed and modified

- **GIVEN** commit `C1` has `old.rs` and commit `C2` has `new.rs` with different content
- **WHEN** diff is computed between `C1` and `C2`
- **THEN** the result SHALL contain a `Removed` entry for `old.rs` and an `Added` entry for `new.rs`
- **AND** no `Renamed` entry SHALL appear (content hashes differ)

#### Scenario: Multiple renames

- **GIVEN** commit `C1` has `a.rs` and `b.rs`, commit `C2` has `x.rs` and `y.rs`
- **AND** `a.rs` content matches `x.rs`, `b.rs` content matches `y.rs`
- **WHEN** diff is computed
- **THEN** the result SHALL contain two `Renamed` entries

### Requirement: Unified diff text rendering

The system SHALL render unified diff text from `DiffEntry` content following standard format.

#### Scenario: Modified file

- **GIVEN** a `DiffEntry` of kind `Modified` with old and new content loaded
- **WHEN** unified diff is rendered with 3 context lines
- **THEN** the output SHALL contain `---`/`+++` headers and `@@` hunk markers
- **AND** removed lines SHALL be prefixed with `-` and added lines with `+`

#### Scenario: Binary file

- **GIVEN** a `DiffEntry` where content is not valid UTF-8
- **WHEN** unified diff is rendered
- **THEN** the output SHALL contain `Binary files a/<path> and b/<path> differ`

#### Scenario: Large file skipped

- **GIVEN** a `DiffEntry` where content was not loaded (exceeded `MAX_DIFF_BLOB_SIZE`)
- **WHEN** unified diff is rendered
- **THEN** the output SHALL show the hash change without line-level diff

### Requirement: CLI diff command

The system SHALL provide `aspen-cli forge diff` for viewing diffs.

#### Scenario: Two-ref diff

- **GIVEN** a running cluster with repo `R`
- **WHEN** `aspen-cli forge diff R heads/main heads/feature` is run
- **THEN** stdout SHALL contain unified diff text for all changed files

#### Scenario: Single-ref diff (show HEAD)

- **GIVEN** repo `R` where `heads/main` points to commit `C` with parent `P`
- **WHEN** `aspen-cli forge diff R heads/main` is run
- **THEN** stdout SHALL show the diff between `P` and `C`

#### Scenario: Stat mode

- **WHEN** `aspen-cli forge diff R heads/main heads/feature --stat` is run
- **THEN** stdout SHALL show a diffstat summary (paths, insertions, deletions)
- **AND** no line-level diff SHALL appear

#### Scenario: Name-only mode

- **WHEN** `aspen-cli forge diff R heads/main heads/feature --name-only` is run
- **THEN** stdout SHALL list only the changed file paths, one per line
