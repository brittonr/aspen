## ADDED Requirements

### Requirement: c2e index keyed by SHA-1

The c2e index SHALL use the git SHA-1 hex string as key instead of blake3 content hash. The KV key format SHALL be `forge:c2e:{repo_hex}:{sha1_hex}` with value `envelope_blake3_hex`.

#### Scenario: Import writes c2e with SHA-1 key

- **WHEN** a git object is imported via `import_objects()` during a git push
- **THEN** the c2e index entry is written with key `forge:c2e:{repo_hex}:{sha1_hex}` where sha1_hex is the object's git SHA-1

#### Scenario: Export looks up c2e by SHA-1

- **WHEN** the federation exporter calls `export_git_objects()` and has c2e entries
- **THEN** it looks up each exported object's SHA-1 (from `converter.export_object()`) in the c2e index to find the envelope BLAKE3 hash

#### Scenario: c2e lookup succeeds for all object types

- **WHEN** a repo with blobs, trees, and commits is imported via git push, then exported via federation sync
- **THEN** every c2e entry written during import SHALL be found during export lookup (100% hit rate for objects present in both paths)

### Requirement: have_set uses SHA-1 domain for git objects

The `collect_local_blake3_hashes()` function SHALL return SHA-1 hashes (zero-padded to 32 bytes) for federation sync have_set population. The exporter SHALL interpret have_set entries as SHA-1 when performing c2e lookup.

#### Scenario: Incremental sync sends SHA-1 have_hashes

- **WHEN** bob has previously imported git objects from alice and requests a second sync
- **THEN** bob's have_hashes contain SHA-1 hashes (zero-padded to 32 bytes) of locally imported objects

#### Scenario: Exporter converts have_set to envelope hashes via c2e

- **WHEN** the exporter receives have_hashes containing SHA-1 identifiers
- **THEN** it looks up each SHA-1 in the c2e index to obtain the corresponding envelope BLAKE3 hash for DAG walk dedup

#### Scenario: Incremental sync skips known objects

- **WHEN** bob syncs from alice after already having imported N objects
- **THEN** the exporter skips objects whose SHA-1 appears in bob's have_set, returning only new objects
