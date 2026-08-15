## Tasks

- [x] [serial] r[molten.artifacts.dependency_edge_records] Define canonical dependency-edge records for artifact, schema, policy, effect, capability, handler profile, storage, transcript, migration, and release relationships.
- [x] [serial] r[molten.artifacts.reverse_dependency_index] Build deterministic reverse dependency indexes from registry and ledger edge records with rebuild receipts.
- [x] [parallel] r[molten.artifacts.impact_query_receipts] Add impact query receipts for direct/transitive dependents, relation filters, redaction decisions, stale-index diagnostics, and planning consumers.
- [x] [parallel] r[molten.artifacts.index_rebuild_determinism] Validate that rebuilding from the same artifact/ledger inputs produces identical sorted edges, reverse indexes, and index digests.
- [x] [serial] r[molten.artifacts.dependency_index_validation] Add positive and negative fixtures for complete graphs, missing edges, duplicate edges, cycles, stale indexes, and unauthorized hidden dependency exposure.