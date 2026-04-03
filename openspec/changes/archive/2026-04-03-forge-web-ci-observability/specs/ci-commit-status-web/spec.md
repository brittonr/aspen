## ADDED Requirements

### Requirement: CI status badges on commit detail page

The forge web SHALL display CI status badges on the commit detail page (`/{repo_id}/commit/{hash}`). Statuses SHALL be fetched by scanning the Forge KV prefix `forge:status:{repo_hex}:{commit_hex}:`. Each `CommitStatus` entry SHALL be rendered as a badge with the context name and state (pending, success, failure, error).

#### Scenario: Commit with passing CI

- **WHEN** user views a commit detail page for a commit with a `CommitStatus` entry where state is `Success` and context is `ci/pipeline`
- **THEN** the page SHALL display a green badge reading "ci/pipeline: passed" in the commit header
- **AND** the badge SHALL link to the pipeline run detail page using the `pipeline_run_id` from the status entry

#### Scenario: Commit with failing CI

- **WHEN** user views a commit detail page for a commit with a `CommitStatus` entry where state is `Failure`
- **THEN** the page SHALL display a red badge reading "ci/pipeline: failed" in the commit header
- **AND** the badge SHALL link to the pipeline run detail page

#### Scenario: Commit with multiple CI contexts

- **WHEN** user views a commit detail page for a commit with `CommitStatus` entries for both `ci/pipeline` and `ci/deploy`
- **THEN** the page SHALL display one badge per context, each with its own state and color

#### Scenario: Commit with no CI status

- **WHEN** user views a commit detail page for a commit with no `CommitStatus` entries
- **THEN** no CI badge section SHALL be displayed

### Requirement: AppState method for commit statuses

`AppState` SHALL expose a `get_commit_statuses(repo_id, commit_hash)` method that scans the `forge:status:{repo_hex}:{commit_hex}:` KV prefix and returns a `Vec<CommitStatus>`. The method SHALL return an empty vec on scan failure rather than propagating the error, since CI status is supplementary information.

#### Scenario: Scan returns statuses

- **WHEN** `get_commit_statuses` is called for a commit with two status entries
- **THEN** it SHALL return a vec containing both `CommitStatus` structs

#### Scenario: Scan fails silently

- **WHEN** `get_commit_statuses` is called and the KV scan returns an error
- **THEN** it SHALL return an empty vec
- **AND** SHALL NOT propagate the error to the caller
