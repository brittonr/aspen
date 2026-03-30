## MODIFIED Requirements

### Requirement: Fetch all objects for a 30K+ object repo

When git-remote-aspen fetches from a federated mirror of a repo with 30,000+ objects, the fetch response SHALL deliver all objects without "Could not read" errors.

#### Scenario: Federated clone of Aspen workspace

- **WHEN** a federated git clone is performed against a mirror with 33,977 objects
- **THEN** git successfully indexes all objects
- **AND** `git log` shows the full commit history
- **AND** no "Could not read" errors appear in git output

#### Scenario: No stuck objects after federation sync

- **WHEN** `sync_from_origin` completes for a 34K object repo
- **THEN** the DAG integrity diagnostic shows stored_count == reachable_count
- **AND** zero missing objects are reported
