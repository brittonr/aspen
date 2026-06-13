# local evaluation cache Delta Spec

## ADDED Requirements

### Requirement: Define canonical `eval-cache-key-v1` records with operation kind, version, input ref, dependency closure hash/refs, handler profile ref, policy/capability/revocation refs, tool ref/version, assumptions, and checks
r[molten.local_eval_cache.key_dto] Define canonical `eval-cache-key-v1` records with operation kind, version, input ref, dependency closure hash/refs, handler profile ref, policy/capability/revocation refs, tool ref/version, assumptions, and checks.

### Requirement: Define canonical `eval-cache-value-v1` records with key ref, deterministic tier, status, output inline/content ref/none, dependencies, policy refs, evidence refs, diagnostics, and checks
r[molten.local_eval_cache.value_dto] Define canonical `eval-cache-value-v1` records with key ref, deterministic tier, status, output inline/content ref/none, dependencies, policy refs, evidence refs, diagnostics, and checks.

### Requirement: Implement and validate `pure`, `simulated`, `policy-current`, and `production-effectful-trace-only` tiers with trace-only semantic-get denial
r[molten.local_eval_cache.determinism_tiers] Implement and validate `pure`, `simulated`, `policy-current`, and `production-effectful-trace-only` tiers with trace-only semantic-get denial.

### Requirement: Document and test that mutable names, file mtimes, local paths, cwd, env vars, and wall-clock time are not sufficient cache identity
r[molten.local_eval_cache.no_name_keys] Document and test that mutable names, file mtimes, local paths, cwd, env vars, and wall-clock time are not sufficient cache identity.

### Requirement: Add a Redb-backed local cache index for key/value records plus reverse indexes by operation, dependency, policy, capability, revocation, evidence, status, tier, and receipts
r[molten.local_eval_cache.redb_index] Add a Redb-backed local cache index for key/value records plus reverse indexes by operation, dependency, policy, capability, revocation, evidence, status, tier, and receipts.

### Requirement: Make derived cache indexes rebuildable from canonical key/value records while preserving historical receipts
r[molten.local_eval_cache.index_rebuild] Make derived cache indexes rebuildable from canonical key/value records while preserving historical receipts.

### Requirement: Emit and parse canonical `eval-cache-receipt-v1` records for put, get, hit, miss, invalidate, stale-deny, and trace-only operations
r[molten.local_eval_cache.receipt_dto] Emit and parse canonical `eval-cache-receipt-v1` records for put, get, hit, miss, invalidate, stale-deny, and trace-only operations.

### Requirement: Store large canonical outputs through chunk/content refs while preserving output refs in cache values
r[molten.local_eval_cache.large_outputs] Store large canonical outputs through chunk/content refs while preserving output refs in cache values.

### Requirement: Implement local cache put/get APIs that validate key/value refs, tier/status compatibility, evidence refs, and output integrity
r[molten.local_eval_cache.put_get] Implement local cache put/get APIs that validate key/value refs, tier/status compatibility, evidence refs, and output integrity.

### Requirement: Revalidate `policy-current` entries against caller-supplied current policy/capability/revocation refs and emit stale-deny receipts on mismatch
r[molten.local_eval_cache.policy_current_revalidation] Revalidate `policy-current` entries against caller-supplied current policy/capability/revocation refs and emit stale-deny receipts on mismatch.

### Requirement: Permit deterministic negative-result caching only when denial/evidence refs are represented in the key and value
r[molten.local_eval_cache.negative_results] Permit deterministic negative-result caching only when denial/evidence refs are represented in the key and value.

### Requirement: Implement invalidation/tombstone by explicit key, dependency ref, policy ref, capability ref, revocation ref, or operation kind
r[molten.local_eval_cache.invalidation] Implement invalidation/tombstone by explicit key, dependency ref, policy ref, capability ref, revocation ref, or operation kind.

### Requirement: Add helper keys/values for schema structural fingerprint results using normalized shape refs and schema-identity tool refs
r[molten.local_eval_cache.schema_fingerprint_cache] Add helper keys/values for schema structural fingerprint results using normalized shape refs and schema-identity tool refs.

### Requirement: Add helper keys/values for schema compatibility decisions over expected/actual identity refs, alias/migration refs, and policy refs
r[molten.local_eval_cache.schema_compat_cache] Add helper keys/values for schema compatibility decisions over expected/actual identity refs, alias/migration refs, and policy refs.

### Requirement: Add helper keys/values for artifact dependency closure results using artifact registry closure hashes
r[molten.local_eval_cache.artifact_closure_cache] Add helper keys/values for artifact dependency closure results using artifact registry closure hashes.

### Requirement: Add API placeholders for Wasm inspection and deterministic transcript-run cache keys without treating production effects as semantic cache hits
r[molten.local_eval_cache.future_wasm_transcript_hooks] Add API placeholders for Wasm inspection and deterministic transcript-run cache keys without treating production effects as semantic cache hits.

### Requirement: Add `molten test cache put` and `get` commands with receipt output and full ref display
r[molten.local_eval_cache.cli_put_get] Add `molten test cache put` and `get` commands with receipt output and full ref display.

### Requirement: Add `status`, `list`, `show`, and `invalidate` commands with filters by operation/tier/status/dependency/policy refs
r[molten.local_eval_cache.cli_status_invalidate] Add `status`, `list`, `show`, and `invalidate` commands with filters by operation/tier/status/dependency/policy refs.

### Requirement: Add tests for unchanged key hits, changed input/dependency misses, output integrity, and no-name-key behavior
r[molten.local_eval_cache.hit_miss_tests] Add tests for unchanged key hits, changed input/dependency misses, output integrity, and no-name-key behavior.

### Requirement: Add tests for policy-current stale denial, deterministic negative-result caching, and trace-only semantic-get denial
r[molten.local_eval_cache.policy_trace_negative_tests] Add tests for policy-current stale denial, deterministic negative-result caching, and trace-only semantic-get denial.

### Requirement: Add Hegel properties for key determinism, dependency invalidation monotonicity, policy-current ref binding, and no-name-key invariants
r[molten.local_eval_cache.property_tests] Add Hegel properties for key determinism, dependency invalidation monotonicity, policy-current ref binding, and no-name-key invariants.

