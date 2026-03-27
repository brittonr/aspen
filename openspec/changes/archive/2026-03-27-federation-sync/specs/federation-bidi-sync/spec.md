## ADDED Requirements

### Requirement: Bidirectional sync transfers objects in both directions

The system SHALL compare local and remote ref heads for a specified repo, pull objects for refs where the remote is ahead or local is missing, and push objects for refs where local is ahead or remote is missing. Both directions SHALL use a single QUIC connection.

#### Scenario: Remote has refs local lacks

- **WHEN** the remote cluster has `heads/main` and the local cluster does not
- **THEN** sync pulls the ref and its objects into a local mirror and reports `pulled > 0`

#### Scenario: Local has refs remote lacks

- **WHEN** the local cluster has `heads/feature` and the remote cluster does not
- **THEN** sync pushes the ref and its objects to the remote and reports `pushed > 0`

#### Scenario: Both sides have unique refs

- **WHEN** local has `heads/feature` (remote lacks it) and remote has `heads/experiment` (local lacks it)
- **THEN** sync pulls `heads/experiment` and pushes `heads/feature`, reporting non-zero counts in both directions

#### Scenario: All refs already in sync

- **WHEN** local and remote ref heads match for all refs
- **THEN** sync reports `pulled: 0`, `pushed: 0`, and no errors

### Requirement: Conflict resolution on divergent refs

When a ref exists on both sides with different hashes and no ancestry information, the system SHALL classify it as a conflict. The default resolution SHALL be pull-biased (remote wins). A `--push-wins` flag SHALL invert this to local wins.

#### Scenario: Divergent ref resolved by remote wins (default)

- **WHEN** local `heads/main` hash differs from remote `heads/main` hash
- **THEN** sync pulls the remote version, overwriting the local ref, and reports the ref in `conflicts`

#### Scenario: Divergent ref resolved by local wins (--push-wins)

- **WHEN** `--push-wins` is set and local `heads/main` hash differs from remote
- **THEN** sync pushes the local version to the remote and reports the ref in `conflicts`

### Requirement: Ref diff computation is a pure function

The ref comparison logic SHALL be implemented as a deterministic pure function with no I/O. It SHALL accept two `HashMap<String, [u8; 32]>` (local heads, remote heads) and return categorized ref lists: `to_pull`, `to_push`, `in_sync`, and `conflicts`.

#### Scenario: Pure function categorizes refs correctly

- **WHEN** local has `{a: H1, b: H2}` and remote has `{a: H1, c: H3}`
- **THEN** `in_sync = [a]`, `to_pull = [c]`, `to_push = [b]`, `conflicts = []`

#### Scenario: Divergent ref appears in conflicts

- **WHEN** local has `{main: H1}` and remote has `{main: H2}` where `H1 != H2`
- **THEN** `conflicts = [main]`, `to_pull = []`, `to_push = []`

### Requirement: Partial failure does not abort the entire sync

If the push half fails (e.g., remote doesn't trust us), the pull half SHALL still succeed. The response SHALL report errors from the failed direction without marking the entire operation as failed.

#### Scenario: Push rejected but pull succeeds

- **WHEN** the remote cluster does not trust us for push but serves objects for pull
- **THEN** sync reports `pulled > 0`, `pushed: 0`, the push error in `errors`, and `is_success: true`

### Requirement: CLI extends existing sync command

The `federation sync` CLI command SHALL accept `--repo <hex>` to trigger bidirectional sync. Without `--repo`, behavior is unchanged (discovery only). `--push-wins` SHALL be available when `--repo` is set.

#### Scenario: Sync with --repo triggers bidi sync

- **WHEN** user runs `federation sync --peer <id> --repo <hex>`
- **THEN** the system performs bidirectional sync and reports pull/push stats

#### Scenario: Sync without --repo does discovery only

- **WHEN** user runs `federation sync --peer <id>` without `--repo`
- **THEN** the system performs discovery (handshake + list resources) as before
