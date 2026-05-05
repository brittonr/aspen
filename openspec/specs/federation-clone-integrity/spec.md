# Federation Clone Integrity

## Purpose

Defines the Federation Clone Integrity capability requirements preserved by Aspen's archived OpenSpec records.

## Requirements

### FCLONE-1: Export BFS must not silently drop objects

When `export_commit_dag` encounters an object BLAKE3 hash that cannot be read from KV or iroh-blobs and cannot be resolved via remap, it MUST return an error rather than silently continuing. The error MUST include the unresolvable hash(es) and the count of successfully collected objects.

#### Scenario: Export BFS must not silently drop objects is enforced

- **WHEN** federation clone integrity processing reaches this condition
- **THEN** the documented requirement MUST be enforced without silently losing object integrity

### FCLONE-2: DAG integrity verification after federation import

After `federation_import_objects` completes and mirror refs are updated, the system MUST verify that all objects reachable from ref heads exist in the mirror's KV store. Missing objects MUST be logged at warn level with their BLAKE3 hashes.

#### Scenario: DAG integrity verification after federation import is enforced

- **WHEN** federation clone integrity processing reaches this condition
- **THEN** the documented requirement MUST be enforced without silently losing object integrity

### FCLONE-3: gpgsig header round-trip preservation

Commits with gpgsig (or other multi-line extra headers) MUST round-trip through import→export with identical SHA-1. The extra_headers field MUST preserve header names, values, and multi-line continuation formatting.

#### Scenario: gpgsig header round-trip preservation is enforced

- **WHEN** federation clone integrity processing reaches this condition
- **THEN** the documented requirement MUST be enforced without silently losing object integrity

### FCLONE-4: Federation import→export completeness

Given a complete set of git objects imported via the federation path, `export_commit_dag` MUST return all objects reachable from the HEAD commit. The count of exported objects MUST equal the count of imported objects (minus any objects not reachable from HEAD).

#### Scenario: Federation import→export completeness is enforced

- **WHEN** federation clone integrity processing reaches this condition
- **THEN** the documented requirement MUST be enforced without silently losing object integrity
