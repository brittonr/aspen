## Tasks

- [x] [serial] r[molten.iroh_discovery.locator_records] Define canonical locator announcement, query, result, and probe receipt records for signed peer claims, tracker responses, pkarr resolutions, static peers, and catalog hints.
- [x] [serial] r[molten.iroh_discovery.hint_only_boundary] Gate federation imports so locator, tracker, pkarr, endpoint, topic, and probe evidence remain hint-only until receiver-driven fetch, hash verification, and local admission pass.
- [x] [parallel] r[molten.iroh_discovery.pkarr_optional_locator] Add optional pkarr-style latest-pointer resolution as locator evidence with signer, freshness, and resolved-ref diagnostics.
- [x] [parallel] r[molten.remote_sync.deterministic_traversal_descriptor] Define deterministic traversal descriptors for artifact closures, chunk manifests, job DAG outputs, sequences, filters, visited sets, and inline policy.
- [x] [parallel] r[molten.remote_sync.traversal_missing_set] Implement pure missing-set and expected-ref planning from traversal descriptors and local registry/chunk summaries.
- [x] [parallel] r[molten.remote_sync.external_digest_mapping] Validate any non-BLAKE3 external digest mapping only after bytes match both the external digest and Molten's BLAKE3 content ref.
- [x] [parallel] r[molten.chunk_store.traversal_sync_strategy] Add stem-first, leaf-only, partitioned, and resumable chunk-manifest sync planning while preserving canonical manifest identity.
- [x] [parallel] r[molten.chunk_store.remote_byte_source_hints] Model S3/HTTP/outboard-style remote byte locations as verified readback hints, not object identity or authority.
- [x] [parallel] r[molten.node_runtime.http3_iroh_readback_adapter] Add an optional HTTP/3-over-Iroh readback adapter boundary that translates to canonical operator gateway requests and receipts.
- [x] [serial] r[molten.iroh_experiments.adoption_validation] Add positive and negative fixtures for locator discovery, deterministic traversal, resumable sync, optional pkarr pointers, remote byte-source readback, HTTP/3 adapter non-authority, malformed inputs, stale evidence, mismatched hashes, and locator-only import denial.
- [x] [serial] r[molten.iroh_experiments.reference_docs] Document `n0-computer/iroh-experiments` as a design reference and state which parts are adopted, deferred, or rejected.
