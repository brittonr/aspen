# Content Addressed Chunk Store Delta: Traversal Sync and Remote Byte Hints

## ADDED Requirements

### Requirement: Chunk-store traversal sync strategies
r[molten.chunk_store.traversal_sync_strategy] Molten SHOULD plan chunk-manifest sync with deterministic traversal strategies such as stem-first metadata fetch, leaf-only data fetch, peer-partitioned leaf fetch, and resumable missing-chunk fetch while preserving canonical manifest identity.

#### Scenario: Stem-first sync preserves manifest identity
- GIVEN a large chunk manifest whose branch metadata and leaf chunks can be fetched separately
- WHEN Molten plans a stem-first sync
- THEN the plan fetches only the selected metadata refs first
- AND the final manifest ref remains the canonical identity after leaves are fetched and verified.

#### Scenario: Leaf partitioning does not duplicate completed chunks
- GIVEN multiple candidate peers and a destination store with some verified chunks already present
- WHEN Molten plans partitioned leaf fetches
- THEN fetch effects are emitted only for missing chunks assigned by the deterministic plan
- AND already verified chunks are not fetched again.

#### Scenario: Traversal strategy denies on manifest drift
- GIVEN fetched metadata changes the expected manifest tree or chunk refs from the traversal descriptor
- WHEN Molten validates the sync plan against fetched refs
- THEN the sync denies before reconstructing or exposing object bytes.

### Requirement: Remote byte-source hints are not identity
r[molten.chunk_store.remote_byte_source_hints] Molten MAY record S3, HTTP, gateway, or other remote byte-source locations and outboard verification metadata as location hints, but MUST treat canonical manifests and chunk refs as object identity.

#### Scenario: Remote range read verifies before bytes are exposed
- GIVEN a remote byte-source hint for a visible manifest range
- WHEN an operator gateway reads that range
- THEN Molten verifies the relevant chunk refs, lengths, transforms, and reconstructed range before returning bytes
- AND the gateway receipt binds the source hint and verification evidence.

#### Scenario: Changed remote object denies readback
- GIVEN a remote source now returns bytes that do not match the expected chunk refs or manifest root
- WHEN Molten attempts readback or import
- THEN it emits a deny receipt
- AND no mismatched bytes are exposed, pinned, installed, or executed.

#### Scenario: Location hint cannot pin or delete content
- GIVEN a remote byte-source hint is present in a manifest or catalog record
- WHEN a subsystem evaluates retention, pinning, deletion, or import
- THEN it requires normal manifest, retention, authority, policy, and evidence gates
- AND the location hint alone grants no mutation authority.

### Requirement: Chunk traversal sync has positive and negative coverage
r[molten.chunk_store.traversal_sync_tests] Molten SHOULD test deterministic chunk traversal planning with positive cases for stem-first sync, leaf-only sync, partitioned fetch, and resumable missing chunks, plus negative cases for manifest drift, stale source hints, corrupt chunks, unsupported transforms, and duplicate or unexpected chunks.

#### Scenario: Resumable sync leaves no missing chunks
- GIVEN a destination store with a partial verified manifest and a deterministic fetch plan
- WHEN the sync completes successfully
- THEN the destination has all chunks required by the manifest
- AND the receipt records which chunks were already present and which were fetched.

#### Scenario: Unexpected chunk is rejected
- GIVEN a sender or remote source returns a chunk not requested by the deterministic plan
- WHEN Molten validates the response
- THEN the unexpected chunk is denied or ignored
- AND it is not indexed as verified content.
