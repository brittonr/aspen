## MODIFIED Requirements

### Requirement: Federated fetch objects

When git-remote-aspen issues `fetch` commands for a federated URL, the local cluster SHALL serve git objects from the local mirror, syncing from the origin first if the mirror lacks the requested objects. The sync SHALL use a single persistent QUIC bidirectional stream for the multi-round object transfer. After sync, the mirror's Forge DAG SHALL be structurally complete so that a BFS walk from any ref head reaches all reachable objects.

#### Scenario: Fetch objects from populated mirror

- **WHEN** the git client requests objects by SHA-1 and the local mirror contains all of them
- **THEN** the local cluster returns the objects from the mirror without contacting the origin

#### Scenario: Fetch objects triggers sync

- **WHEN** the git client requests objects that the local mirror does not have
- **THEN** the local cluster performs a federation sync for the missing objects, imports them into the mirror, and returns them

#### Scenario: Fetch all objects for a 30K+ object repo

- **WHEN** the git client fetches from a federated mirror of a repo with 33,897 objects
- **THEN** the fetch response contains all 33,897 objects
- **AND** git successfully indexes all objects without "Could not read" errors

#### Scenario: Sync uses single QUIC stream

- **WHEN** the federation sync transfers objects in 17 rounds
- **THEN** all 17 request/response exchanges occur on the same bidirectional stream
- **AND** no stream exhaustion or reconnection occurs
