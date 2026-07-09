# Local Evaluation Cache Specification

## Purpose

Defines the `local-evaluation-cache` capability.

## Requirements

### Requirement: System MUST Define canonical `eval-cache-key-v1` records with operation kind, version, input ref, dependency closure hash/refs, handler profile ref, policy/capability/revocation refs, tool ref/version, assumptions, and checks
r[molten.local_eval_cache.key_dto] The system MUST Define canonical `eval-cache-key-v1` records with operation kind, version, input ref, dependency closure hash/refs, handler profile ref, policy/capability/revocation refs, tool ref/version, assumptions, and checks.

### Requirement: System MUST Define canonical `eval-cache-value-v1` records with key ref, deterministic tier, status, output inline/content ref/none, dependencies, policy refs, evidence refs, diagnostics, and checks
r[molten.local_eval_cache.value_dto] The system MUST Define canonical `eval-cache-value-v1` records with key ref, deterministic tier, status, output inline/content ref/none, dependencies, policy refs, evidence refs, diagnostics, and checks.

### Requirement: System MUST Implement and validate `pure`, `simulated`, `policy-current`, and `production-effectful-trace-only` tiers with trace-only semantic-get denial
r[molten.local_eval_cache.determinism_tiers] The system MUST Implement and validate `pure`, `simulated`, `policy-current`, and `production-effectful-trace-only` tiers with trace-only semantic-get denial.

### Requirement: System MUST Document and test that mutable names, file mtimes, local paths, cwd, env vars, and wall-clock time are not sufficient cache identity
r[molten.local_eval_cache.no_name_keys] The system MUST Document and test that mutable names, file mtimes, local paths, cwd, env vars, and wall-clock time are not sufficient cache identity.

### Requirement: System MUST Add a Redb-backed local cache index for key/value records plus reverse indexes by operation, dependency, policy, capability, revocation, evidence, status, tier, and receipts
r[molten.local_eval_cache.redb_index] The system MUST Add a Redb-backed local cache index for key/value records plus reverse indexes by operation, dependency, policy, capability, revocation, evidence, status, tier, and receipts.

### Requirement: System MUST Make derived cache indexes rebuildable from canonical key/value records while preserving historical receipts
r[molten.local_eval_cache.index_rebuild] The system MUST Make derived cache indexes rebuildable from canonical key/value records while preserving historical receipts.

### Requirement: System MUST Emit and parse canonical `eval-cache-receipt-v1` records for put, get, hit, miss, invalidate, stale-deny, and trace-only operations
r[molten.local_eval_cache.receipt_dto] The system MUST Emit and parse canonical `eval-cache-receipt-v1` records for put, get, hit, miss, invalidate, stale-deny, and trace-only operations.

### Requirement: System MUST Store large canonical outputs through chunk/content refs while preserving output refs in cache values
r[molten.local_eval_cache.large_outputs] The system MUST Store large canonical outputs through chunk/content refs while preserving output refs in cache values.

### Requirement: System MUST Implement local cache put/get APIs that validate key/value refs, tier/status compatibility, evidence refs, and output integrity
r[molten.local_eval_cache.put_get] The system MUST Implement local cache put/get APIs that validate key/value refs, tier/status compatibility, evidence refs, and output integrity.

### Requirement: System MUST Revalidate `policy-current` entries against caller-supplied current policy/capability/revocation refs and emit stale-deny receipts on mismatch
r[molten.local_eval_cache.policy_current_revalidation] The system MUST Revalidate `policy-current` entries against caller-supplied current policy/capability/revocation refs and emit stale-deny receipts on mismatch.

### Requirement: System MUST Permit deterministic negative-result caching only when denial/evidence refs are represented in the key and value
r[molten.local_eval_cache.negative_results] The system MUST Permit deterministic negative-result caching only when denial/evidence refs are represented in the key and value.

### Requirement: System MUST Implement invalidation/tombstone by explicit key, dependency ref, policy ref, capability ref, revocation ref, or operation kind
r[molten.local_eval_cache.invalidation] The system MUST Implement invalidation/tombstone by explicit key, dependency ref, policy ref, capability ref, revocation ref, or operation kind.

### Requirement: System MUST Add helper keys/values for schema structural fingerprint results using normalized shape refs and schema-identity tool refs
r[molten.local_eval_cache.schema_fingerprint_cache] The system MUST Add helper keys/values for schema structural fingerprint results using normalized shape refs and schema-identity tool refs.

### Requirement: System MUST Add helper keys/values for schema compatibility decisions over expected/actual identity refs, alias/migration refs, and policy refs
r[molten.local_eval_cache.schema_compat_cache] The system MUST Add helper keys/values for schema compatibility decisions over expected/actual identity refs, alias/migration refs, and policy refs.

### Requirement: System MUST Add helper keys/values for artifact dependency closure results using artifact registry closure hashes
r[molten.local_eval_cache.artifact_closure_cache] The system MUST Add helper keys/values for artifact dependency closure results using artifact registry closure hashes.

### Requirement: System MUST Add API placeholders for Wasm inspection and deterministic transcript-run cache keys without treating production effects as semantic cache hits
r[molten.local_eval_cache.future_wasm_transcript_hooks] The system MUST Add API placeholders for Wasm inspection and deterministic transcript-run cache keys without treating production effects as semantic cache hits.

### Requirement: System MUST Add `molten test cache put` and `get` commands with receipt output and full ref display
r[molten.local_eval_cache.cli_put_get] The system MUST Add `molten test cache put` and `get` commands with receipt output and full ref display.

### Requirement: System MUST Add `status`, `list`, `show`, and `invalidate` commands with filters by operation/tier/status/dependency/policy refs
r[molten.local_eval_cache.cli_status_invalidate] The system MUST Add `status`, `list`, `show`, and `invalidate` commands with filters by operation/tier/status/dependency/policy refs.

### Requirement: System MUST Add tests for unchanged key hits, changed input/dependency misses, output integrity, and no-name-key behavior
r[molten.local_eval_cache.hit_miss_tests] The system MUST Add tests for unchanged key hits, changed input/dependency misses, output integrity, and no-name-key behavior.

### Requirement: System MUST Add tests for policy-current stale denial, deterministic negative-result caching, and trace-only semantic-get denial
r[molten.local_eval_cache.policy_trace_negative_tests] The system MUST Add tests for policy-current stale denial, deterministic negative-result caching, and trace-only semantic-get denial.

### Requirement: System MUST Add Hegel properties for key determinism, dependency invalidation monotonicity, policy-current ref binding, and no-name-key invariants
r[molten.local_eval_cache.property_tests] The system MUST Add Hegel properties for key determinism, dependency invalidation monotonicity, policy-current ref binding, and no-name-key invariants.

### Requirement: rkyv archives preserve Preserves source of truth
r[molten.local_eval_cache.rkyv_preserves_source_of_truth] rkyv-backed zero-copy archives MUST be treated as derived local cache materializations of canonical Preserves cache keys, values, traces, or indexes rather than as canonical source artifacts.

#### Scenario: Derived archive is rebuilt from canonical records
- GIVEN canonical Preserves cache key and value records exist for a cached result
- WHEN a rkyv archive sidecar is missing, stale, or discarded
- THEN Molten can rebuild or deny cache acceleration from the canonical records without changing the semantic cache identity

#### Scenario: Archive bytes are not accepted as source evidence
- GIVEN only a rkyv archive file is available without matching canonical Preserves source refs
- WHEN cache admission evaluates the file
- THEN admission denies semantic cache reads until canonical source records or a rebuild path are supplied

### Requirement: Derived archive manifests are canonical Preserves records
r[molten.local_eval_cache.rkyv_manifest] Each rkyv-backed derived archive SHOULD be described by a canonical Preserves manifest that binds cache purpose, archive schema/profile version, producer tool ref, canonical source refs, BLAKE3 source digests, archive byte digest, validation requirement, validation receipt ref, rebuild capability, and retention class.

#### Scenario: Manifest binds archive to current source refs
- GIVEN a manifest names source refs and source digests for a derived archive
- WHEN cache admission compares it with caller-supplied current source refs
- THEN admission only permits archive reads when the refs and digests match the current canonical sources

#### Scenario: Bare archive has no cache authority
- GIVEN a rkyv archive has no manifest
- WHEN cache admission evaluates it
- THEN the archive is treated as disposable bytes and cannot satisfy a cache hit

### Requirement: rkyv archive admission is pure before shell IO
r[molten.local_eval_cache.rkyv_admission] Molten MUST decide derived archive usability with a pure core over manifest facts and current canonical source refs before shell code reads, mmaps, rebuilds, or exposes rkyv archive data.

#### Scenario: Pure admission requests rebuild
- GIVEN a manifest source digest differs from the current canonical Preserves source digest
- WHEN the pure admission core evaluates the manifest
- THEN it returns a rebuild or deny decision before any archive read controls semantic output

### Requirement: rkyv archive reads require validation evidence
r[molten.local_eval_cache.rkyv_validation] rkyv archives loaded from disk, peer exchange, bundles, or other untrusted storage MUST be validated for the exact archive bytes before safe archived reads are exposed to cache callers.

#### Scenario: Validated archive can accelerate read
- GIVEN a manifest is admitted and validation passes for the exact archive byte digest
- WHEN a local cache caller requests the derived view
- THEN the shell may expose a read-only archived view while keeping canonical Preserves refs as the semantic identity

#### Scenario: Validation failure denies archive use
- GIVEN a rkyv archive fails byte validation, alignment checks, or exact-byte receipt matching
- WHEN the cache shell attempts to use it
- THEN it denies or rebuilds the archive before returning semantic cache data

### Requirement: rkyv archive bytes are not canonical identity
r[molten.local_eval_cache.rkyv_identity_boundary] Cache keys, cache values, receipts, evidence refs, policy refs, release refs, and storage refs MUST continue to derive canonical identity from Preserves content and declared BLAKE3 source inputs rather than from rkyv archive byte layout.

#### Scenario: Archive layout changes without semantic identity change
- GIVEN the same canonical Preserves sources are materialized with a different rkyv layout or producer version
- WHEN cache identity is computed
- THEN the semantic cache key and evidence refs remain bound to the canonical Preserves sources, while the derived archive manifest records the changed materialization

#### Scenario: Attempted archive identity overclaim is rejected
- GIVEN a manifest claims that a rkyv archive byte digest is the authoritative cache key or evidence ref
- WHEN cache admission validates the manifest
- THEN admission fails closed with an identity-boundary diagnostic

### Requirement: Derived archive behavior has positive and negative tests
r[molten.local_eval_cache.rkyv_negative_tests] rkyv-derived cache behavior SHOULD include positive tests for admitted current manifests and rebuildable archives, plus negative tests for stale source refs, wrong BLAKE3 digests, archive tampering, missing validation receipts, malformed manifests, incompatible archive profiles, and attempts to treat derived archives as authoritative.

#### Scenario: Tampered archive fails before cache hit
- GIVEN a manifest names an archive byte digest
- WHEN archive bytes are changed after manifest creation
- THEN cache validation rejects the archive before reporting a cache hit

#### Scenario: Missing validation receipt fails closed
- GIVEN an untrusted archive lacks validation evidence for its exact bytes
- WHEN a caller requests the archived view
- THEN the cache shell denies or rebuilds instead of reading through unchecked archive data
