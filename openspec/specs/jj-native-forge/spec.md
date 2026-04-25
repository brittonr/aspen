# jj-native-forge Specification

## Purpose
TBD - created by archiving change jj-native-forge-wasm-plugin. Update Purpose after archive.
## Requirements
### Requirement: JJ-native Forge backend is delivered as a WASM plugin

The JJ-native Forge backend SHALL be delivered as a registered WASM plugin. Its manifest SHALL declare the JJ request families, JJ protocol identifiers, JJ storage permissions, and session resource limits required for JJ-native repo access.

#### Scenario: Register JJ Forge plugin

- **WHEN** an administrator installs the JJ-native Forge backend
- **THEN** the cluster registers it as a plugin artifact with declared JJ request families, protocol identifiers, permissions, and resource limits
- **AND** JJ-native repo access remains unavailable until that plugin is present and active

#### Scenario: JJ repo requires plugin availability

- **WHEN** a repository is marked JJ-enabled but the JJ-native Forge plugin is not active on the target node
- **THEN** JJ-native access fails with a capability-unavailable error
- **AND** the system does not silently replace the JJ backend with a host-native fallback path

### Requirement: JJ objects are stored natively in Forge

Aspen Forge SHALL store Jujutsu objects without translating them through the Git SHA-1 bridge. JJ commits, trees, files, conflicts, and related metadata SHALL be persisted as BLAKE3-addressed blobs with Raft-backed indexes for repo membership and reachability.

#### Scenario: Push JJ objects into Forge

- **WHEN** a JJ client pushes a change containing new commits, trees, and files
- **THEN** Forge stores the pushed JJ objects as native JJ payloads addressed by BLAKE3
- **AND** no SHA-1 mapping table is required for those JJ objects

#### Scenario: Fetch JJ objects from another node

- **WHEN** a follower or peer node needs JJ objects for a repo it does not yet cache locally
- **THEN** the node fetches the missing JJ blobs through Aspen's blob distribution path
- **AND** the fetched objects remain valid for JJ-native reads without Git re-encoding

#### Scenario: Conflict objects round-trip losslessly

- **WHEN** a JJ conflict object is pushed, stored, and later fetched through the JJ-native path
- **THEN** Forge returns that conflict object in JJ-native form without Git-style normalization or lossy translation
- **AND** the client can reconstruct the same JJ conflict state from the returned payload

### Requirement: JJ payloads are validated before publish

Forge SHALL decode and validate incoming JJ payloads before they become repo-visible state.

#### Scenario: Reject malformed JJ payload

- **WHEN** a JJ client sends a malformed or unsupported JJ object payload during push
- **THEN** Forge rejects the push before final publish
- **AND** the visible JJ bookmarks and change-id index remain unchanged

#### Scenario: Reject inconsistent object graph

- **WHEN** a JJ push references missing or type-mismatched JJ objects during validation
- **THEN** Forge rejects the publish step with a JJ-native error
- **AND** the invalid object graph does not become visible to later JJ reads

### Requirement: JJ state mutation requires repo write authorization

Any JJ publish, bookmark mutation, or change-id mutation path SHALL require repo-scoped write authorization before JJ-visible state can change.

#### Scenario: Unauthorized JJ mutation is rejected

- **WHEN** a caller without repo write permission attempts a JJ publish or bookmark mutation through any backend entry path
- **THEN** Forge rejects the mutation before JJ-visible state changes
- **AND** JJ bookmarks and change-id indexes remain unchanged

#### Scenario: Authorized JJ mutation can proceed

- **WHEN** a caller with repo write permission submits a valid JJ publish or bookmark mutation
- **THEN** Forge may continue to validation and publish checks for that mutation

### Requirement: Forge preserves JJ change identity

Forge SHALL preserve JJ change IDs as first-class repo state. The cluster SHALL maintain a repo-scoped index from change ID to current JJ commit head so clients can resolve and synchronize changes without reconstructing identity from Git metadata.

#### Scenario: Resolve pushed change by change ID

- **WHEN** a JJ client pushes a commit with change ID `qpvuntsm`
- **THEN** Forge records the mapping between that change ID and the stored JJ commit head for the target repo
- **AND** later JJ reads can resolve `qpvuntsm` directly from Forge state

#### Scenario: Change rewrite advances the index

- **WHEN** a JJ client rewrites an existing change and pushes the replacement commit
- **THEN** Forge updates the repo-scoped change-id index to point at the new JJ commit head
- **AND** the superseded commit remains addressable by object hash after the rewrite response completes

### Requirement: Final publish detects stale JJ heads

Before a JJ push becomes repo-visible, Forge SHALL check the expected bookmark heads and change-id heads supplied for that publish. If another publish has already advanced one of those heads, Forge SHALL reject the stale publish with JJ-native conflict information instead of overwriting newer state.

#### Scenario: Concurrent bookmark move is rejected

- **WHEN** two JJ pushes race to update the same bookmark and one publish has already committed first
- **THEN** the later stale publish is rejected with JJ-native conflict information
- **AND** the committed bookmark head remains unchanged by the rejected publish

#### Scenario: Concurrent change rewrite is rejected

- **WHEN** a JJ push tries to advance a change ID from an out-of-date expected head
- **THEN** Forge rejects the stale publish with JJ-native conflict information
- **AND** the newer change-id mapping remains authoritative

### Requirement: JJ bookmark updates go through consensus

JJ bookmark creation, movement, and deletion SHALL be committed through Raft consensus and stored under a JJ-specific namespace separate from Git refs.

#### Scenario: Atomic publish after successful push

- **WHEN** a JJ push finishes validation and is accepted
- **THEN** Forge publishes the new JJ objects, change-id index updates, and bookmark updates as one repo-visible state transition
- **AND** readers do not observe only a subset of that accepted update

#### Scenario: Failed push leaves visible state unchanged

- **WHEN** a JJ push stream fails or is rejected before final publish
- **THEN** Forge leaves the visible JJ bookmarks and change-id index at their pre-push values
- **AND** later retry attempts start from a consistent repo-visible state

#### Scenario: Move bookmark

- **WHEN** a JJ client moves bookmark `main` to a new JJ commit head
- **THEN** the bookmark update is committed through Raft before success is returned
- **AND** the bookmark is stored under the JJ bookmark namespace instead of `refs/heads/`

#### Scenario: Delete bookmark

- **WHEN** a JJ client deletes bookmark `feature-x`
- **THEN** Forge removes the bookmark from the JJ namespace through a consensus-backed write
- **AND** Git refs in the same repo remain unchanged

### Requirement: Git and JJ backends can coexist per repo

A Forge repository SHALL be able to advertise Git support, JJ support, or both. When both are enabled, Git refs and JJ bookmarks/change indexes SHALL remain namespaced so one backend does not corrupt the other.

#### Scenario: Dual-backend repository

- **WHEN** a repository is configured to support both Git and JJ clients
- **THEN** Forge exposes Git refs and JJ bookmark/change state as separate namespaces in the same repo record
- **AND** pushes from one backend do not overwrite the other's refs or indexes

#### Scenario: JJ-only repository

- **WHEN** a repository is created with only JJ support enabled
- **THEN** Forge accepts JJ-native operations for that repo
- **AND** Git remote-helper operations for that repo fail with a capability error instead of partial fallback

#### Scenario: Git behavior remains unchanged on dual-backend repo

- **WHEN** JJ support is enabled for a repository that still advertises Git support
- **THEN** existing Git operations on that repo continue to use the Git backend behavior and namespaces unchanged
- **AND** the presence of the JJ plugin does not redirect Git operations through JJ-native paths
