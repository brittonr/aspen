## ADDED Requirements

### Requirement: Single-stream sync conversation

The federation sync client SHALL reuse a single QUIC bidirectional stream for the entire multi-round object sync conversation, rather than opening a new stream per RPC round.

#### Scenario: Multi-round sync uses one stream

- **WHEN** a federation sync transfers 33,897 objects in 17 rounds of 2,000 objects each
- **THEN** exactly one bidirectional stream is opened for the sync (plus one for the handshake)
- **AND** no "Too many streams" warning appears in the server log

#### Scenario: Server loops on a single stream

- **WHEN** the server accepts a sync stream
- **THEN** it reads requests and writes responses in a loop on that stream until the client finishes the send side

#### Scenario: Capability negotiation for streaming sync

- **WHEN** the client handshakes with a server that does not advertise `streaming-sync` capability
- **THEN** the client falls back to opening a new stream per RPC round

### Requirement: Origin SHA-1 cross-pass resolution in convergent import

`federation_import_objects` SHALL store origin SHA-1 → BLAKE3 mappings in the mapping store after each convergent import pass, not only in Phase 4.

#### Scenario: Tree references subtree by origin SHA-1

- **WHEN** a tree T references subtree S by origin SHA-1 X, and S was imported in pass 1 with re-serialized SHA-1 Y (where X ≠ Y)
- **THEN** pass 1 stores `X → blake3(S)` in the mapping store
- **AND** T imports successfully in pass 2 by resolving X to blake3(S)

#### Scenario: All objects converge with zero stuck

- **WHEN** 33,897 objects are imported via the convergent loop with cross-batch dependencies
- **THEN** the final retry pass reports `errors=0` (no unresolvable dependencies)

### Requirement: Post-import DAG re-resolution

After the final retry pass, `federation_import_objects` SHALL verify and fix BLAKE3 references in the mirror's Forge DAG so that every tree entry points to an object stored under that exact BLAKE3 hash.

#### Scenario: Stale BLAKE3 reference corrected

- **WHEN** tree T was imported in round 3 with entry `blake3=H1` for blob B, but B's actual envelope hash in the mirror is H2 (different from H1 due to mapping state at round 3)
- **THEN** re-resolution updates T's entry to `blake3=H2` and re-stores T with corrected references

#### Scenario: Re-resolution cascades to parent

- **WHEN** re-resolution changes tree T's envelope hash from H_old to H_new
- **THEN** any parent tree or commit referencing T by H_old is also updated to reference H_new

#### Scenario: Full DAG reachable after re-resolution

- **WHEN** re-resolution completes for a mirror with 33,897 imported objects
- **THEN** a BFS DAG walk from HEAD reaches all 33,897 objects (minus any genuinely missing objects, which SHALL be logged as warnings)

#### Scenario: Re-resolution is idempotent

- **WHEN** re-resolution runs on a mirror whose DAG is already correct
- **THEN** zero trees are modified and the operation completes in O(ref_count) time

### Requirement: Federated clone succeeds for large repos

A federated git clone SHALL succeed end-to-end for repositories with 30,000+ objects.

#### Scenario: Clone Aspen workspace via federation

- **WHEN** `git clone aspen://<bob-ticket>/fed:<alice-key>:<repo-id>` runs against a mirror of the Aspen workspace (33,897 objects)
- **THEN** git receives all objects and the clone completes without error
- **AND** the cloned repository has the same HEAD commit SHA-1 as the origin

#### Scenario: Clone after reconnection

- **WHEN** the QUIC connection drops mid-sync and the client reconnects
- **THEN** the sync resumes transferring objects from where it left off (using have_hashes) and the clone still succeeds
