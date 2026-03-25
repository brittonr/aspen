## MODIFIED Requirements

### Requirement: Virtual @branch path routing

`AspenFs` SHALL detect `@branch-name` as the first path component after the mount root. Paths under `/@name/rest/of/path` SHALL resolve through a `BranchOverlay` instance keyed by `name`. Paths without an `@` prefix SHALL resolve through the base store as today. Branch dirty map reads SHALL always be served immediately (no lazy deferral or cache TTL) since they are in-memory. Only base store fall-through reads SHALL go through the lazy fetch and cache validation path.

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

#### Scenario: Branch dirty map reads are immediate

- **WHEN** a branch has key "src/main.rs" in its dirty map
- **AND** a read is issued for `/@branch/src/main.rs`
- **THEN** the data SHALL be returned from the in-memory dirty map immediately
- **AND** no cache lookup or cluster RPC SHALL occur

#### Scenario: Branch fall-through uses lazy fetch

- **WHEN** a branch does NOT have key "src/lib.rs" in its dirty map
- **AND** a read is issued for `/@branch/src/lib.rs`
- **THEN** the read SHALL fall through to the base store
- **AND** the base store read SHALL use the lazy fetch path (content-hash validation, offline fallback)
