# Runtime Spine Delta: retention destructive path gates

### Requirement: Retention-gated destructive paths
r[molten.retention.ledger_gc_gate] Molten MUST gate evidence-ledger garbage collection through passing retention receipts before removing ledger content.

#### Scenario: Ledger GC denied before removal
- GIVEN a ledger artifact with an active retention pin
- WHEN ledger GC evaluates the artifact for removal
- THEN GC emits denial evidence and does not remove the artifact

r[molten.retention.chunk_gc_gate] Molten MUST gate chunk-store manifest and chunk removal through passing retention receipts before removing files or writing tombstone receipts.

#### Scenario: Chunk GC denied before tombstone
- GIVEN an unpinned chunk-store manifest that is still retention-pinned
- WHEN chunk GC evaluates the manifest for removal
- THEN no manifest or chunk is removed and the GC receipt binds the denying retention receipt

r[molten.retention.eval_cache_tombstone_gate] Molten MUST gate evaluation-cache invalidation tombstones through passing retention receipts before writing tombstone entries.

#### Scenario: Cache tombstone denied before mutation
- GIVEN an evaluation-cache key with active retention evidence
- WHEN invalidation selects the key
- THEN the tombstone is not written unless retention eligibility passes

r[molten.retention.secret_cleanup_gate] Molten MUST require secret cleanup receipts to bind actual passing retention receipts for the cleaned secret and tombstone.

#### Scenario: Secret cleanup rejects stale retention evidence
- GIVEN a secret cleanup request with missing or mismatched retention evidence
- WHEN the cleanup receipt is built
- THEN cleanup is denied and diagnostics identify the retention mismatch

### Requirement: Subsystem retention evidence
r[molten.retention.subsystem_receipt_refs] Molten MUST expose retention receipt refs in ledger GC, chunk GC, cache invalidation, and secret cleanup receipts without treating them as authority grants.

#### Scenario: Subsystem receipt binds retention refs
- GIVEN a destructive subsystem decision that evaluated retention
- WHEN the subsystem receipt is emitted
- THEN the receipt lists the retention receipt refs that informed the decision

r[molten.retention.destructive_gate_tests] Molten MUST test pass and fail-closed retention-gated destructive paths for ledger GC, chunk GC, cache tombstones, and secret cleanup.

#### Scenario: Denials leave content intact
- GIVEN bounded generated or fixture destructive candidates with incomplete or denied retention decisions
- WHEN the subsystem attempts cleanup
- THEN tests verify content remains intact and denial receipts are auditable
