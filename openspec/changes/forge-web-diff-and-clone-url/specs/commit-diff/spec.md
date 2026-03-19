## ADDED Requirements

### Requirement: Commit detail page shows file-level changes

The web UI SHALL display a commit detail page at `/{repo_id}/commit/{hash}` showing the commit metadata (message, author, timestamp, hash) and a list of files changed relative to the first parent commit.

#### Scenario: View a commit with one parent

- **WHEN** user navigates to `/{repo_id}/commit/{hash}` for a commit with one parent
- **THEN** the page SHALL display the commit message, author, timestamp, and full hash, and list each file that was added, modified, or deleted compared to the parent

#### Scenario: View a root commit (no parents)

- **WHEN** user navigates to a commit with no parents
- **THEN** all files in the commit's tree SHALL be shown as added

#### Scenario: View a merge commit (multiple parents)

- **WHEN** user navigates to a commit with multiple parents
- **THEN** the diff SHALL compare against the first parent only

### Requirement: Unified diff for text files

The web UI SHALL display a unified diff for each changed text file, showing added and removed lines with context.

#### Scenario: Modified text file

- **WHEN** a file exists in both parent and child trees with different content
- **THEN** the page SHALL render a unified diff with added lines marked green and removed lines marked red, with surrounding context lines

#### Scenario: Added file

- **WHEN** a file exists in the child tree but not the parent
- **THEN** all lines SHALL be shown as additions

#### Scenario: Deleted file

- **WHEN** a file exists in the parent tree but not the child
- **THEN** all lines SHALL be shown as deletions

### Requirement: Diff limits for safety

The system SHALL bound diff computation to prevent resource exhaustion.

#### Scenario: Too many files changed

- **WHEN** a commit changes more than 50 files
- **THEN** the page SHALL show the first 50 file diffs and a message indicating more files were changed

#### Scenario: File too large for inline diff

- **WHEN** a changed file exceeds 256KB
- **THEN** the page SHALL show "Binary or large file — diff not shown" instead of inline diff content

### Requirement: Commit links from log page

The commit log page SHALL link each commit hash to the commit detail page.

#### Scenario: Click commit hash on log page

- **WHEN** user clicks a commit hash on the commits page
- **THEN** the browser SHALL navigate to `/{repo_id}/commit/{hash}`
