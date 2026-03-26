## ADDED Requirements

### Requirement: Server serves git objects via SyncObjects

The `ForgeResourceResolver::sync_objects` SHALL serve git objects (commits, trees, blobs) when `want_types` includes `"commit"`, `"tree"`, or `"blob"`. The server SHALL walk the commit DAG from ref heads and return objects not in the client's `have_hashes` set.

#### Scenario: Client requests git objects for a federated repo

- **WHEN** a client sends `SyncObjects` with `want_types: ["commit", "tree", "blob"]` and empty `have_hashes`
- **THEN** the server walks from ref heads, returns reachable git objects as `SyncObject` entries with correct `object_type`, raw git object `data`, and BLAKE3 `hash`

#### Scenario: Client provides have_hashes for incremental sync

- **WHEN** a client sends `SyncObjects` with `have_hashes` containing BLAKE3 hashes of objects it already has
- **THEN** the server excludes those objects and any objects reachable only through them from the response

#### Scenario: Object count exceeds batch limit

- **WHEN** the reachable object set exceeds `limit` (capped at `MAX_OBJECTS_PER_SYNC = 1000`)
- **THEN** the server returns up to `limit` objects with `has_more: true`, and the client paginates by adding received hashes to `have_hashes` in the next request

### Requirement: Git objects are BLAKE3 verified

Every `SyncObject` returned by the server SHALL have its `hash` field set to the BLAKE3 hash of the `data` field. The client SHALL verify this hash before importing.

#### Scenario: Hash matches data

- **WHEN** the client receives a `SyncObject` with `object_type: "commit"`
- **THEN** the client computes `blake3::hash(data)` and verifies it matches `hash` before importing

#### Scenario: Hash mismatch rejected

- **WHEN** a received `SyncObject` has a `hash` that doesn't match `blake3::hash(data)`
- **THEN** the client drops the object and logs a warning, continuing with remaining objects

### Requirement: DAG walk respects depth limits

The server's DAG walk SHALL respect `MAX_DAG_DEPTH` to prevent unbounded traversal on deep histories.

#### Scenario: Deep history capped

- **WHEN** a repo has more than `MAX_DAG_DEPTH` commits in a linear chain
- **THEN** the server stops traversal at the depth limit and returns `has_more: true`
