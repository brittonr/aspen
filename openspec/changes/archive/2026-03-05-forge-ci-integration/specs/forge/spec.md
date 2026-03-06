## MODIFIED Requirements

### Requirement: Git Object Storage

The system SHALL store Git objects (commits, trees, blobs, tags) as content-addressed blobs via iroh-blobs. Objects SHALL be retrievable by their Git SHA and distributable across cluster nodes. The system SHALL support reading individual files from the tree at a specific commit without full checkout.

#### Scenario: Push objects

- **WHEN** a user pushes a commit with 10 new objects
- **THEN** each Git object SHALL be stored as an iroh-blob
- **AND** reference updates SHALL be committed through Raft consensus

#### Scenario: Fetch objects

- **WHEN** a client performs a Git fetch
- **THEN** the forge SHALL serve them from iroh-blobs
- **AND** objects may be fetched from any node that has them

#### Scenario: Read file at commit

- **WHEN** `read_file_at_commit(repo_id, commit_hash, "path/to/file")` is called
- **THEN** the system SHALL resolve the commit object, walk its tree to the given path, and return the blob contents
- **AND** return `None` if the path does not exist in the tree

#### Scenario: Push emits hook event

- **WHEN** a git push successfully updates refs on a repository
- **THEN** the system SHALL emit a `ForgePushCompleted` hook event with repo_id, ref_name, new_hash, and old_hash
- **AND** hook subscribers SHALL receive the event
