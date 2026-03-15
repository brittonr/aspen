## MODIFIED Requirements

### Requirement: Virtual @branch path routing

`AspenFs` SHALL detect `@branch-name` as the first path component after the mount root. Paths under `/@name/rest/of/path` SHALL resolve through a `BranchOverlay` instance keyed by `name`. Paths without an `@` prefix SHALL resolve through the base store as today. When cached execution tracking is enabled, read tracking SHALL operate on branch-resolved paths, recording the content hash of the value returned (whether from the branch dirty map or the parent store).

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

#### Scenario: Read tracking records branch-resolved content

- **WHEN** cached execution tracking is enabled
- **AND** PID 1234 reads `/@experiment/src/main.rs`
- **AND** the branch dirty map has a value for that key with BLAKE3 hash H1
- **THEN** the read set for PID 1234 SHALL record hash H1
- **AND** the hash SHALL reflect the branch's version of the file, not the base store's version
