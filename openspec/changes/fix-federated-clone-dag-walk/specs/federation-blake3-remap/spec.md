## ADDED Requirements

### Requirement: DAG exporter resolves origin BLAKE3 references via remap index

The `export_commit_dag_collect` BFS SHALL, when a BLAKE3 hash from a commit/tree payload is not found directly in the mirror's object store, consult the remap index (`forge:remap:{repo}:{origin_blake3}`) to translate to the mirror's BLAKE3 hash and continue the walk.

#### Scenario: Tree entry references blob by origin BLAKE3

- **WHEN** a tree object contains an entry with origin BLAKE3 `aaa...` and the mirror stored that blob under BLAKE3 `bbb...`, with remap entry `aaa... → bbb...`
- **THEN** the exporter reads the blob via `bbb...` and includes it in the export result

#### Scenario: Commit references parent by origin BLAKE3

- **WHEN** a commit references parent BLAKE3 `ccc...` which is stored in the mirror as `ddd...`
- **THEN** the exporter follows the remap to `ddd...`, reads the parent commit, and continues the BFS through its dependencies

#### Scenario: Non-federated repo has no remap entries

- **WHEN** the exporter walks a DAG on a non-federated repo (direct push, no federation)
- **THEN** all BLAKE3 references resolve directly from KV without remap lookups, and no remap-related errors occur

#### Scenario: Missing remap entry terminates walk for that branch

- **WHEN** a BLAKE3 reference is not found directly and has no remap entry
- **THEN** the exporter logs a warning with the unresolvedBLAKE3 hash and skips that dependency (does not error the entire export)

### Requirement: Remap index populated during federation import

`federation_import_objects` SHALL write a KV entry `forge:remap:{mirror_repo_hex}:{origin_blake3_hex} → {mirror_blake3_hex}` for each successfully imported object whose `SyncObject.envelope_hash` is present.

#### Scenario: Object with envelope_hash gets remap entry

- **WHEN** a SyncObject with `envelope_hash = Some(origin_hash)` is imported into the mirror, producing mirror BLAKE3 `mirror_hash`
- **THEN** KV contains `forge:remap:{repo}:{origin_hash_hex} = {mirror_hash_hex}`

#### Scenario: Object without envelope_hash skips remap

- **WHEN** a SyncObject with `envelope_hash = None` is imported
- **THEN** no remap entry is written, and a debug log is emitted

#### Scenario: Post-sync convergent retry writes remap entries

- **WHEN** the convergent retry loop imports 15,000 objects that failed in per-batch passes
- **THEN** all 15,000 objects have remap entries in KV

#### Scenario: Re-import overwrites remap entry idempotently

- **WHEN** the same object is imported twice (e.g., re-sync)
- **THEN** the remap entry is overwritten with the same value, no error occurs

### Requirement: Full DAG export for federated mirror repos

After a successful federation sync and convergent import, `handle_git_bridge_fetch` on the mirror SHALL return all objects reachable from the requested commit, matching the count exported by the origin.

#### Scenario: Full clone of 33K-object repo via federation

- **WHEN** origin has 33,847 objects, federation sync imports all of them into the mirror, and git-remote-aspen requests a fetch of HEAD
- **THEN** the response contains all 33,847 objects and `git clone` succeeds without "Could not read" errors

#### Scenario: Incremental fetch after partial clone

- **WHEN** a mirror has objects from a previous sync and a new sync adds 500 objects
- **THEN** a fetch requesting the new HEAD returns only the new objects (old objects are in the `have` set)
