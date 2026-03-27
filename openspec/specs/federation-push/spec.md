## ADDED Requirements

### Requirement: Origin cluster can push objects to a remote cluster

The origin cluster SHALL send git objects and ref updates to a remote cluster over the federation sync protocol. The remote cluster SHALL import the objects and update mirror refs.

#### Scenario: Push a single-commit repo

- **WHEN** Alice's cluster pushes repo objects (1 blob, 1 tree, 1 commit) and ref `heads/main` to Bob's cluster
- **THEN** Bob's cluster imports all 3 objects and creates a mirror ref `heads/main` pointing to the imported commit

#### Scenario: Incremental push after new commit

- **WHEN** Alice pushes a second commit (with `have_hashes` from the first push excluded), sending only new objects
- **THEN** Bob's cluster imports only the new objects and updates `heads/main` to the new commit

### Requirement: Push requires receiver trust

The remote cluster SHALL reject push requests from untrusted clusters.

#### Scenario: Push from untrusted cluster

- **WHEN** an untrusted cluster sends a `PushObjects` request
- **THEN** the receiver returns an error response with code `"unauthorized"`

#### Scenario: Push from trusted cluster

- **WHEN** a trusted cluster sends a `PushObjects` request with valid objects
- **THEN** the receiver accepts and imports the objects

### Requirement: Push respects object limits

The push request SHALL be rejected if it exceeds `MAX_OBJECTS_PER_SYNC`.

#### Scenario: Push exceeding object limit

- **WHEN** a push request contains more than `MAX_OBJECTS_PER_SYNC` objects
- **THEN** the receiver returns an error response with code `"limit_exceeded"`

### Requirement: Push creates mirror repo on first contact

When the receiver has no existing mirror for the pushed `fed_id`, it SHALL create a new mirror repo and import objects into it.

#### Scenario: First push to a new cluster

- **WHEN** Alice pushes to Bob who has never seen this `fed_id`
- **THEN** Bob creates a mirror repo derived from the `fed_id`, imports all objects, and sets the mirror refs

### Requirement: CLI push command

The CLI SHALL provide `aspen-cli federation push --peer <key> --repo <id>` to push a local repo to a remote cluster.

#### Scenario: CLI push invocation

- **WHEN** the user runs `aspen-cli federation push --peer <bob-key> --repo <repo-id>`
- **THEN** the CLI exports the repo's objects and refs, connects to Bob's cluster, and pushes them
- **AND** the CLI prints the number of objects imported and refs updated

## MODIFIED Requirements

### Requirement: Push requires explicit authorization

The federation push handler SHALL verify that the remote peer has explicit push authorization for the target resource. Authorization SHALL be granted by either TrustManager trust level OR a session credential containing a `FederationPush` capability matching the target resource. Checking credential _presence_ without verifying its capabilities SHALL NOT be sufficient.

#### Scenario: Push accepted from trusted peer without credential

- **WHEN** a remote peer is in the TrustManager's trusted set
- **AND** the peer does not present a credential
- **THEN** the push SHALL be accepted (backwards compatible)

#### Scenario: Push accepted from untrusted peer with valid FederationPush credential

- **WHEN** a remote peer is not in the TrustManager's trusted set (Public trust level)
- **AND** the peer presents a credential with `FederationPush { repo_prefix }` matching the target resource
- **THEN** the push SHALL be accepted

#### Scenario: Push rejected from untrusted peer with pull-only credential

- **WHEN** a remote peer presents a credential with only `FederationPull` capabilities
- **AND** the peer is not in the TrustManager's trusted set
- **THEN** the push SHALL be rejected with `unauthorized`
- **AND** the error message SHALL indicate push capability is required

#### Scenario: Push rejected from blocked peer regardless of credential

- **WHEN** a remote peer is in the TrustManager's blocked set
- **AND** the peer presents a valid credential with `FederationPush`
- **THEN** the push SHALL be rejected
