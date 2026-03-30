## MODIFIED Requirements

### Requirement: Federated fetch objects

When git-remote-aspen issues `fetch` commands for a federated URL, the local cluster SHALL serve git objects from the local mirror, syncing from the origin first if the mirror lacks the requested objects. For mirrors with more than 2,000 objects, the server SHALL use the chunked fetch protocol to avoid exceeding RPC response size limits.

#### Scenario: Fetch objects from populated mirror

- **WHEN** the git client requests objects by SHA-1 and the local mirror contains all of them
- **THEN** the local cluster returns the objects from the mirror without contacting the origin

#### Scenario: Fetch objects triggers sync

- **WHEN** the git client requests objects that the local mirror does not have
- **THEN** the local cluster performs a federation sync for the missing objects, imports them into the mirror, and returns them

#### Scenario: Fetch all objects for a 30K+ object repo

- **WHEN** the git client fetches from a federated mirror of a repo with 33,897 objects
- **THEN** the fetch response delivers all 33,897 objects via the chunked protocol
- **AND** git successfully indexes all objects without "Could not read" errors

#### Scenario: Response size bounded per chunk

- **WHEN** the server sends a chunk of 2,000 objects
- **THEN** the serialized chunk response is less than 16 MB (the `MAX_CLIENT_MESSAGE_SIZE` limit)
