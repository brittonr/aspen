## Phase 1: Cache model

- [x] [serial] r[molten.eval_cache.key_model] Define domain-separated cache keys with operation kind, input hash, dependency closure hash, handler profile, policy refs, and tool version.
- [x] [serial] r[molten.eval_cache.value_model] Define cache values with status, output hash/content ref, deterministic tier, evidence refs, and diagnostics.
- [x] [serial] r[molten.eval_cache.determinism_tiers] Classify cacheable results as pure, simulated, policy-current, or production-effectful trace-only.
- [x] [parallel] r[molten.eval_cache.no_name_keys] Document that mutable names, file mtimes, and local paths are not sufficient cache keys.

## Phase 2: Storage and evidence

- [x] [serial] r[molten.eval_cache.redb_store] Add a Redb-backed local cache index for cache keys, values, dependencies, and evidence refs.
- [x] [serial] r[molten.eval_cache.receipts] Emit trace/evidence records for cache hits, misses, and cache-influenced decisions.
- [x] [parallel] r[molten.eval_cache.policy_current] Revalidate policy-current cache entries against current policy/capability/revocation inputs before use.
- [x] [parallel] r[molten.eval_cache.negative_results] Cache deterministic negative results only when denial inputs are represented in the key.

## Phase 3: First cached operations

- [x] [serial] r[molten.eval_cache.schema_validation] Cache schema validation results for immutable value/schema hashes.
- [x] [serial] r[molten.eval_cache.choreography_projection] Cache Trellis projectability and endpoint projection results by protocol artifact and dependency closure.
- [x] [parallel] r[molten.eval_cache.wasm_inspection] Cache Wasm/component inspection summaries by module artifact and inspector version.
- [x] [parallel] r[molten.eval_cache.transcript_results] Cache deterministic executable transcript results by transcript artifact, dependency closure, handler profile, and seed/config.

## Phase 4: Tests

- [x] [serial] r[molten.eval_cache.hit_miss_tests] Add tests for cache hit/miss behavior under unchanged and changed canonical inputs.
- [x] [serial] r[molten.eval_cache.policy_revalidation_tests] Add tests that policy-current cache entries are rejected after relevant policy/revocation changes.
- [x] [parallel] r[molten.eval_cache.simulated_handler_tests] Add tests for deterministic local/mock/chaos handler cache keys.
- [x] [parallel] r[molten.eval_cache.property_tests] Add Hegel property tests for key determinism, dependency invalidation, and no-name-key invariants.
