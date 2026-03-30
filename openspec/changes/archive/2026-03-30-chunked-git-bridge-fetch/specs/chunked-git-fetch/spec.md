## ADDED Requirements

### Requirement: Chunked fetch session lifecycle

The server SHALL support a `GitBridgeFetchStart` → N × `GitBridgeFetchChunk` → `GitBridgeFetchComplete` protocol for transferring large object sets.

#### Scenario: Start a chunked fetch session

- **WHEN** a client sends `GitBridgeFetchStart { repo_id, want, have }`
- **THEN** the server walks the commit DAG, creates a session, and responds with `{ session_id, total_objects, total_chunks }`

#### Scenario: Retrieve a chunk

- **WHEN** a client sends `GitBridgeFetchChunk { session_id, chunk_id }`
- **THEN** the server responds with the objects for that chunk and a BLAKE3 content hash for integrity verification

#### Scenario: Complete a chunked fetch

- **WHEN** a client sends `GitBridgeFetchComplete { session_id }`
- **THEN** the server cleans up the session state

#### Scenario: Session expires

- **WHEN** a session is idle for more than 5 minutes
- **THEN** subsequent chunk requests return an error with code `SESSION_EXPIRED`

### Requirement: Automatic chunked mode for large repos

The server SHALL automatically use chunked fetch when the DAG walk produces more than 2,000 objects.

#### Scenario: Small repo uses single-shot fetch

- **WHEN** a `GitBridgeFetch` request walks a DAG with 500 objects
- **THEN** the server responds with all 500 objects inline in a single `GitBridgeFetchResponse`

#### Scenario: Large repo returns fetch-start redirect

- **WHEN** a `GitBridgeFetch` request walks a DAG with 33,897 objects
- **THEN** the server responds with `GitBridgeFetchResponse { is_success: true, objects: [], chunked_session_id: Some(id), total_objects, total_chunks }` signaling the client to switch to chunked mode

### Requirement: Chunk batching by object type

Chunks SHALL be ordered with blobs first, then trees, then commits, so the client can write loose objects in dependency order.

#### Scenario: First chunks contain blobs

- **WHEN** the server batches 33,897 objects into chunks
- **THEN** the first N chunks contain only blob objects, followed by tree chunks, then commit chunks

### Requirement: Client incremental writing

git-remote-aspen SHALL write objects to `.git/objects/` incrementally per chunk, never accumulating all objects in memory.

#### Scenario: Incremental write during chunked fetch

- **WHEN** git-remote-aspen receives a chunk with 2,000 objects
- **THEN** it writes those 2,000 objects as loose files before requesting the next chunk

#### Scenario: Memory bounded

- **WHEN** git-remote-aspen fetches 33,897 objects in 17 chunks
- **THEN** peak memory usage is bounded by one chunk (~4 MB) plus overhead, not by the total object set (~490 MB)

### Requirement: Federation internal chunked fetch

The federation git fetch handler SHALL use the chunked fetch path for large mirror repos, assembling the response internally without RPC overhead.

#### Scenario: Federation fetch of large mirror

- **WHEN** `handle_federation_git_fetch` serves a mirror with 33,897 objects
- **THEN** it uses the chunked handler internally and returns `FederationGitFetch` with all objects (or triggers the client to use chunked RPCs)
