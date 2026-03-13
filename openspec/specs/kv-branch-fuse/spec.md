## ADDED Requirements

### Requirement: Virtual @branch path routing

`AspenFs` SHALL detect `@branch-name` as the first path component after the mount root. Paths under `/@name/rest/of/path` SHALL resolve through a `BranchOverlay` instance keyed by `name`. Paths without an `@` prefix SHALL resolve through the base store as today.

#### Scenario: Read through @branch path

- **WHEN** a branch named "experiment" exists
- **AND** a read is issued for `/@experiment/src/main.rs`
- **THEN** the read SHALL resolve through the "experiment" `BranchOverlay`
- **AND** the overlay SHALL fall through to the base store for keys not in the branch

#### Scenario: Write through @branch path

- **WHEN** a write is issued to `/@experiment/src/main.rs` with content "new code"
- **THEN** the write SHALL buffer in the "experiment" branch's dirty map
- **AND** the base store SHALL NOT be modified

#### Scenario: Base path unaffected

- **WHEN** a read is issued for `/src/main.rs` (no `@` prefix)
- **THEN** the read SHALL resolve through the base store directly
- **AND** no branch overlay SHALL be consulted

#### Scenario: Invalid branch name

- **WHEN** a path references `/@nonexistent/file`
- **AND** no branch named "nonexistent" exists
- **THEN** the operation SHALL return ENOENT

### Requirement: Branch lifecycle via control file

Each `@branch` virtual directory SHALL contain a `.branchfs_ctl` virtual file. Writing "commit" to this file SHALL commit the branch. Writing "abort" SHALL discard the branch.

#### Scenario: Commit via control file

- **WHEN** "commit" is written to `/@experiment/.branchfs_ctl`
- **THEN** the "experiment" branch SHALL be committed to the base store
- **AND** the `/@experiment/` virtual directory SHALL be removed

#### Scenario: Abort via control file

- **WHEN** "abort" is written to `/@experiment/.branchfs_ctl`
- **THEN** the "experiment" branch SHALL be dropped
- **AND** no writes from the branch SHALL reach the base store
- **AND** the `/@experiment/` virtual directory SHALL be removed

#### Scenario: Create branch via control file

- **WHEN** "create:feature-x" is written to `/.branchfs_ctl` (root control file)
- **THEN** a new branch named "feature-x" SHALL be created
- **AND** `/@feature-x/` SHALL become accessible

### Requirement: Readdir lists @branch directories

The root directory listing SHALL include `@branch-name` entries for all active branches, alongside normal directory entries from the base store.

#### Scenario: Branches appear in root listing

- **WHEN** branches "agent-a" and "agent-b" exist
- **AND** the base store has directories "src" and "docs"
- **THEN** `readdir("/")` SHALL return ["@agent-a", "@agent-b", "src", "docs"]

#### Scenario: Branch directory contents reflect overlay

- **WHEN** branch "agent-a" has a dirty write for "src/new-file.rs"
- **AND** the base store has "src/main.rs"
- **THEN** `readdir("/@agent-a/src/")` SHALL return ["main.rs", "new-file.rs"]

### Requirement: Branch cleanup on unmount

All active branches SHALL be dropped (aborted) when the FUSE filesystem is unmounted. No uncommitted branch state SHALL persist after unmount.

#### Scenario: Unmount drops all branches

- **WHEN** branches "a", "b", "c" exist with uncommitted writes
- **AND** the filesystem is unmounted
- **THEN** all three branches SHALL be dropped
- **AND** no writes from any branch SHALL reach the base store
