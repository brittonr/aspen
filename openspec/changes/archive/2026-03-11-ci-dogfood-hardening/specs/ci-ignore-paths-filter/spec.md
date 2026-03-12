## ADDED Requirements

### Requirement: Trigger evaluates ignore_paths against changed files

The `TriggerService` SHALL compare the list of files changed between `old_hash` and `new_hash` against the `ignore_paths` glob patterns in the pipeline config. If every changed file matches an ignore pattern, the pipeline SHALL NOT be triggered.

#### Scenario: Docs-only push is skipped

- **WHEN** a push updates only `README.md` and `docs/architecture.md`
- **AND** the pipeline config has `ignore_paths = ["*.md", "docs/*"]`
- **THEN** the trigger SHALL be skipped
- **AND** a debug log SHALL indicate the push was filtered by ignore_paths

#### Scenario: Mixed push triggers normally

- **WHEN** a push updates `README.md` and `src/main.rs`
- **AND** the pipeline config has `ignore_paths = ["*.md"]`
- **THEN** the trigger SHALL proceed (not all files match ignore patterns)

#### Scenario: First push always triggers

- **WHEN** a push has `old_hash = None` (first push to a ref)
- **THEN** the trigger SHALL always proceed regardless of `ignore_paths`
- **AND** no tree diff SHALL be attempted

#### Scenario: Large diff falls back to trigger

- **WHEN** the tree diff between `old_hash` and `new_hash` exceeds 10,000 changed entries
- **THEN** the trigger SHALL proceed without filtering (safe default)
- **AND** a warning SHALL be logged indicating the diff was too large to filter

### Requirement: Tree diff uses Forge git objects

The file diff SHALL be computed by comparing the tree objects at `old_hash` and `new_hash` using `ForgeNode.git` methods. No git checkout SHALL be required for diffing.

#### Scenario: Diff identifies changed files

- **WHEN** commit A has tree `{src/main.rs, README.md}` and commit B adds `src/lib.rs`
- **THEN** the diff SHALL report `src/lib.rs` as a changed file
- **AND** `README.md` and `src/main.rs` SHALL NOT appear in the diff (unchanged)

#### Scenario: Diff handles nested directories

- **WHEN** a file at `crates/aspen-ci/src/worker.rs` is modified
- **THEN** the diff SHALL report the full path `crates/aspen-ci/src/worker.rs`
