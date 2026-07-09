# Content Addressed Chunk Store Specification

## Purpose

Adds capability-bounded local filesystem roots for Aspen chunk-store, retention, dataspace, and exchange boundaries.

## Requirements

### Requirement: Local chunk-store filesystem authority is capability-rooted
r[molten.chunk_store.cap_std_boundary.root_wrappers] Molten MUST represent local artifact, chunk, retention, dataspace, and exchange filesystem authority as typed capability roots opened by the outer runtime or adapter shell.

#### Scenario: Valid local store path stays under the declared root
r[molten.chunk_store.cap_std_boundary.tests.positive]
- GIVEN Molten has opened an operator-declared local store root
- WHEN chunk-store, retention, dataspace, or exchange code reads or writes a valid relative path through the typed root
- THEN the operation MUST remain confined to that declared root while preserving canonical chunk and manifest identity behavior.

#### Scenario: Invalid local store path cannot escape the root
r[molten.chunk_store.cap_std_boundary.tests.negative]
- GIVEN input names `../` traversal, an absolute path, a missing capability root, a symlink escape attempt, or a remote locator presented as a local path
- WHEN Molten resolves the locator for local filesystem access through the typed root
- THEN Molten MUST fail closed before reading, writing, deleting, or exposing local artifact bytes outside the declared root.

### Requirement: Capability dependency stays outside identity cores
r[molten.chunk_store.cap_std_boundary.dependency] Molten MUST add `cap-std` only to modules that own local filesystem effects and MUST keep chunk, manifest, catalog, and identity cores independent of capability filesystem APIs.

#### Scenario: Manifest and catalog logic remains filesystem-neutral
r[molten.chunk_store.cap_std_boundary.conversion]
- GIVEN manifest, chunk-ref, catalog, and visibility-policy logic is reviewed
- WHEN cap-std adoption is complete
- THEN those cores MUST operate on in-memory content refs, DTOs, or validated relative locators and MUST NOT open ambient filesystem paths directly.

### Requirement: Capability boundary is documented without artifact overclaims
r[molten.chunk_store.cap_std_boundary.docs] Molten MUST document that capability roots bound local filesystem authority only and do not prove artifact truth, confidentiality, remote transport trust, Merkle correctness, or distributed runtime correctness.

#### Scenario: Closeout evidence includes local boundary validation
r[molten.chunk_store.cap_std_boundary.validation]
- GIVEN the cap-std artifact-store change is ready to archive
- WHEN focused chunk-store, retention, dataspace, exchange, Cairn validation, and gate checks run
- THEN the evidence MUST include positive and negative filesystem-boundary coverage and visible non-claim documentation.
