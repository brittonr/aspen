## ADDED Requirements

### Requirement: Discussion creation

The system SHALL allow creating standalone discussion threads on a repository. Each discussion SHALL be a COB with type `Discussion`, stored as an immutable change DAG in iroh-blobs with heads tracked in Raft KV.

#### Scenario: Create a discussion

- **WHEN** a user creates a discussion on repo `R` with title `"RFC: New API design"` and body `"Let's discuss the v2 API..."`
- **THEN** a new COB of type Discussion SHALL be created with a unique ID
- AND the discussion SHALL have state `Open`
- AND the discussion SHALL have zero replies

#### Scenario: Create discussion with invalid title

- **WHEN** a user creates a discussion with a title exceeding `MAX_TITLE_LENGTH_BYTES`
- **THEN** the system SHALL return `ForgeError::InvalidCobChange`

### Requirement: Discussion replies

The system SHALL support adding replies to a discussion. Replies MAY reference a parent reply to form threads.

#### Scenario: Add a top-level reply

- **WHEN** a user adds a reply with body `"I think option A is better"` to discussion `D`
- **THEN** the reply SHALL be appended to the discussion's reply list
- AND the reply SHALL have no parent reference
- AND the discussion's `updated_at_ms` SHALL advance

#### Scenario: Add a threaded reply

- **WHEN** a user adds a reply with `parent_reply` set to reply `R1`
- **THEN** the reply SHALL reference `R1` as its parent
- AND the reply SHALL appear in the flat reply list ordered by HLC timestamp

### Requirement: Discussion state resolution

The system SHALL resolve the current state of a discussion by walking its change DAG, topologically sorting changes, and applying operations in causal order. The resolved state SHALL include title, body, state, replies, labels, and reactions.

#### Scenario: Resolve discussion with multiple replies

- **WHEN** discussion `D` has a creation change and 3 reply changes
- **THEN** `resolve_discussion` SHALL return a `Discussion` with 3 entries in `replies`
- AND replies SHALL be ordered by HLC timestamp

#### Scenario: Resolve discussion with concurrent changes

- **WHEN** two users concurrently add labels `"rfc"` and `"api"` to discussion `D`
- **THEN** resolution SHALL produce a discussion with both labels (union merge)

### Requirement: Discussion thread resolution

The system SHALL support marking individual reply threads as resolved, indicating the sub-conversation is complete.

#### Scenario: Resolve a thread

- **WHEN** a user resolves the thread rooted at reply `R1`
- **THEN** `R1` SHALL be marked as resolved in the discussion state
- AND the discussion SHALL remain open

#### Scenario: Unresolve a thread

- **WHEN** a user unresolves a previously resolved thread `R1`
- **THEN** `R1` SHALL no longer be marked as resolved

### Requirement: Discussion locking

The system SHALL support locking a discussion to prevent new replies. Locked discussions SHALL still be readable.

#### Scenario: Lock a discussion

- **WHEN** a delegate locks discussion `D`
- **THEN** the discussion state SHALL be `Locked`
- AND attempts to add replies SHALL return `ForgeError::DiscussionLocked`

#### Scenario: Unlock a discussion

- **WHEN** a delegate unlocks a locked discussion `D`
- **THEN** the discussion state SHALL return to `Open`
- AND replies SHALL be accepted again

### Requirement: Discussion closing

The system SHALL support closing and reopening discussions.

#### Scenario: Close a discussion

- **WHEN** a user closes discussion `D` with reason `"Decided on option A"`
- **THEN** the discussion state SHALL be `Closed`

#### Scenario: Reopen a closed discussion

- **WHEN** a user reopens a closed discussion `D`
- **THEN** the discussion state SHALL return to `Open`

### Requirement: Discussion listing

The system SHALL support listing discussions in a repository, filtered by state.

#### Scenario: List open discussions

- **WHEN** a user lists discussions for repo `R` with state filter `Open`
- **THEN** only discussions with state `Open` SHALL be returned
- AND results SHALL be ordered by `updated_at_ms` descending

#### Scenario: List all discussions

- **WHEN** a user lists discussions with state filter `All`
- **THEN** all discussions (open, closed, locked) SHALL be returned

### Requirement: Discussion CLI commands

The CLI SHALL provide `discussion` subcommands for managing discussions.

#### Scenario: Create discussion via CLI

- **WHEN** a user runs `aspen-cli discussion create --repo R --title "RFC" --body "..."`
- **THEN** a new discussion SHALL be created and its ID printed

#### Scenario: List discussions via CLI

- **WHEN** a user runs `aspen-cli discussion list --repo R`
- **THEN** open discussions SHALL be listed with ID, title, reply count, and last activity

#### Scenario: Reply via CLI

- **WHEN** a user runs `aspen-cli discussion reply --repo R DISCUSSION_ID --body "..."`
- **THEN** a reply SHALL be added to the discussion

### Requirement: Discussion RPC protocol

The client API SHALL include RPC request/response types for all discussion operations.

#### Scenario: ForgeCreateDiscussion RPC

- **WHEN** a client sends `ForgeCreateDiscussion { repo_id, title, body, labels }`
- **THEN** the node SHALL create the discussion and return `{ discussion_id }`

#### Scenario: ForgeListDiscussions RPC

- **WHEN** a client sends `ForgeListDiscussions { repo_id, state_filter, limit }`
- **THEN** the node SHALL return a list of resolved discussion summaries
