## ADDED Requirements

### Requirement: Federation git object export succeeds

The federation sync protocol SHALL return git object content (commits, trees, blobs) when `sync_objects` is called with `want_types` including git object types, provided the objects exist in the origin cluster's blob store.

#### Scenario: Export objects for a pushed repo

- **WHEN** a repo has objects pushed via git-remote-aspen and the repo is federated
- **THEN** `sync_objects` with `want_types: ["commit", "tree", "blob"]` returns all reachable objects with non-empty `data` fields

#### Scenario: Export after fresh import

- **WHEN** objects are imported via `GitImporter::import_object` and immediately exported via `GitExporter::export_object`
- **THEN** the export succeeds without `encode error` or `ObjectNotFound`

### Requirement: Diagnostic logging on blob read failure

The exporter SHALL log the blob hash, whether the blob exists (`has()` check), and the specific iroh-blobs error when `get_bytes` fails, so the root cause is identifiable from node logs.

#### Scenario: Blob read fails with encode error

- **WHEN** `export_object` calls `get_bytes` and receives an `encode error`
- **THEN** the log message includes the BLAKE3 hash, the `has()` result, and the raw error string

### Requirement: Federated git clone produces working tree

After fixing blob export, the federation git clone NixOS VM test SHALL verify that Bob's cloned repo contains the actual file contents from Alice's repo.

#### Scenario: Clone content matches

- **WHEN** Bob clones Alice's federated repo via `git clone aspen://<ticket>/fed:...`
- **THEN** `cat README.md` in the clone matches the content Alice committed
