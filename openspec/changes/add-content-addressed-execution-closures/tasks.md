## Phase 1: Model and inventory

- [ ] [serial] Inventory current job, CI, runtime-host, hook, and deploy payload identity fields and choose the first executor slice.
- [ ] [depends:inventory] Define the versioned closure manifest, canonical hash input, dependency graph fields, schema-hash fields, runtime target, provenance, and compatibility mapping for the selected executor.

## Phase 2: Admission and transfer

- [ ] [depends:manifest] Add closure validation and negative tests for malformed manifests, schema mismatches, and unsupported runtime targets.
- [ ] [depends:validation] Wire dependency fetch/verification for the selected executor using existing blob/Iroh transfer paths.
- [ ] [depends:fetch] Wire capability/effect admission before execution and preserve deny-before-runtime-start behavior.

## Phase 3: Receipts and proof

- [ ] [depends:admission] Emit closure execution receipts with closure hash, dependency handle, runtime target, schema hashes, input/output handles, status, and redacted capability summary.
- [ ] [depends:receipts] Add product-path positive proof and negative tests for missing dependency and denied capability.
- [ ] [depends:tests] Update operator/developer docs and run focused tests, strict OpenSpec validation, and `git diff --check`.
