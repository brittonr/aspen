## ADDED Requirements

### Requirement: Chunk availability state is proof checked
r[molten.chunk_cache_state_proof.chunk_availability] Molten MUST prove that chunk manifests, chunk entries, availability indexes, partial fetch receipts, and missing scans agree before serving or reconstructing content.

#### Scenario: Corrupt fetched chunk denies read
- GIVEN a manifest whose fetched chunk bytes hash to the wrong chunk ref
- WHEN chunk reconstruction is requested
- THEN the read or fetch receipt decision is `deny`
- AND no reconstructed artifact ref is emitted.

### Requirement: Chunk GC requires exact retention gates
r[molten.chunk_cache_state_proof.retention_gc] Molten MUST prove that chunk and manifest GC removes content only when matching retention apply and execution gate refs bind the same object ref, object kind, action, and retention class.

#### Scenario: Missing apply ref denies chunk removal
- GIVEN an unpinned chunk candidate and no matching retention apply ref
- WHEN non-dry-run chunk GC is requested
- THEN GC decision is `deny`
- AND the chunk remains present or marked unavailable without deletion.
