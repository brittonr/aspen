## ADDED Requirements

### Requirement: Artifact listing on job log page

The forge web SHALL display an "Artifacts" section on the job log page (`/{repo_id}/ci/{run_id}/{job_id}`) below the log output. The section SHALL list artifacts returned by `CiListArtifacts` for that job. Each artifact entry SHALL display the artifact name, size in human-readable format (bytes, KB, MB), and content type.

#### Scenario: Job with artifacts

- **WHEN** user views a job log page for a completed job that produced artifacts
- **THEN** the page SHALL display an "Artifacts" section listing each artifact with name, size, and content type
- **AND** the section SHALL appear below the log viewer

#### Scenario: Job with no artifacts

- **WHEN** user views a job log page for a job that produced no artifacts
- **THEN** the "Artifacts" section SHALL NOT be displayed

#### Scenario: Artifacts for a running job

- **WHEN** user views a job log page for a job with status "running"
- **THEN** the artifacts section SHALL show whatever artifacts are available so far
- **AND** new artifacts SHALL appear on the next auto-refresh

### Requirement: Artifact CLI download hint

Each artifact entry SHALL display a CLI download command (`aspen-cli ci artifact <blob_hash>`) as a copyable code block, since direct HTTP download requires the h3-to-blob proxy which is not yet available.

#### Scenario: Artifact download hint

- **WHEN** user views an artifact entry
- **THEN** the entry SHALL include a code block with `aspen-cli ci artifact <blob_hash>`
