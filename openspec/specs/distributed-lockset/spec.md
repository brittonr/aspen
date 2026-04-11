## ADDED Requirements

### Requirement: Atomic lock-set acquisition

The system SHALL provide a distributed `LockSet` primitive that acquires a bounded set of named lock resources atomically. An acquire attempt either claims every requested resource or claims none of them.

#### Scenario: Successful atomic acquisition

- **WHEN** a client acquires a lock set for resources `repo:a`, `pipeline:42`, and `deploy:prod` and all three resources are absent, released, or expired
- **THEN** the acquire attempt succeeds with one guard covering all three resources
- **AND** the system records ownership for all three resources in one linearizable state transition

#### Scenario: Try-acquire returns immediately on contention

- **WHEN** a client calls `try_acquire` for the lock set `[repo:a, pipeline:42]`
- **AND** `pipeline:42` is currently held by another live holder
- **THEN** the operation returns without retrying
- **AND** the system reports that the full set was not acquired
- **AND** no lock state is modified

#### Scenario: Acquire retries until the set becomes available

- **WHEN** a client calls retrying `acquire` for the lock set `[repo:a, pipeline:42]`
- **AND** one member is initially held by another live holder and later released before the acquire timeout expires
- **THEN** the operation retries with bounded backoff
- **AND** it eventually acquires the full set with one guard before the timeout expires

#### Scenario: Contended member prevents partial acquisition

- **WHEN** a client acquires a lock set for resources `repo:a`, `pipeline:42`, and `deploy:prod`
- **AND** `pipeline:42` is currently held by another live holder
- **THEN** the acquire attempt fails without claiming any subset of the requested resources

### Requirement: Canonicalized member ordering and validation

The system SHALL canonicalize lock-set members by rejecting duplicates, enforcing a fixed maximum member count, and sorting the validated resource names into a stable total order before evaluation.

#### Scenario: Equivalent requests converge on one member order

- **WHEN** client A requests the set `[pipeline:42, repo:a]`
- **AND** client B requests the set `[repo:a, pipeline:42]`
- **THEN** the system evaluates both requests against the same canonical member ordering
- **AND** at most one client acquires the full set at a time

#### Scenario: Duplicate member rejected

- **WHEN** a client requests the set `[repo:a, repo:a]`
- **THEN** the system rejects the request as invalid
- **AND** no lock state is modified

#### Scenario: Oversized lock set rejected

- **WHEN** a client requests more members than `MAX_LOCKSET_KEYS` allows
- **THEN** the system rejects the request as invalid
- **AND** no lock state is modified

### Requirement: Per-resource fencing tokens

The system SHALL assign and persist a fencing token for each resource in a lock set. Any resource that is reacquired after release or expiry SHALL receive a token strictly greater than its previous token.

#### Scenario: Reacquire after release increments token

- **WHEN** client A acquires and releases a lock set containing `repo:a`
- **AND** client B later acquires a lock set containing `repo:a`
- **THEN** client B receives a fencing token for `repo:a` that is strictly greater than client A's token for `repo:a`

#### Scenario: Overlapping sets preserve per-resource monotonicity

- **WHEN** client A acquires and releases the set `[repo:a, pipeline:42]`
- **AND** client B later acquires the set `[repo:a, deploy:prod]`
- **THEN** the new fencing token for `repo:a` is strictly greater than its previous token
- **AND** `deploy:prod` receives its own token according to its prior history

### Requirement: Lock-set TTL, renewal, and release

The system SHALL apply TTL expiration, renewal, and release to the full lock set as one unit.

#### Scenario: Renew extends every member

- **WHEN** a client renews an active lock set containing `repo:a` and `pipeline:42`
- **THEN** the deadline for both resources is extended using the lock-set TTL
- **AND** neither resource changes holder or fencing token during the renew operation

#### Scenario: Release transitions every member together

- **WHEN** a client releases an active lock set containing `repo:a` and `pipeline:42`
- **THEN** both resources transition to released state in one atomic operation
- **AND** a later acquire attempt may claim both resources without observing a partially released set

#### Scenario: Expired lock set can be taken over

- **WHEN** every member of a previously acquired lock set has expired
- **THEN** a new client may acquire the full set
- **AND** the acquire attempt assigns fresh fencing tokens to the claimed resources

### Requirement: Remote coordination API for lock sets

The system SHALL expose lock-set acquisition, renewal, and release through Aspen's coordination RPC surface so remote clients can use the primitive without bespoke lock choreography.

#### Scenario: Remote client acquires a lock set

- **WHEN** an Aspen client calls the coordination API to acquire a lock set for `[repo:a, pipeline:42]`
- **THEN** the server returns a lock-set guard representation containing the canonical member list and per-resource fencing tokens

#### Scenario: Remote client renews and releases the same guard

- **WHEN** the same Aspen client later renews and releases that lock-set guard through the coordination API
- **THEN** the server applies the operation to the same canonical member set
- **AND** the release succeeds only if the guard still matches the currently held resources
