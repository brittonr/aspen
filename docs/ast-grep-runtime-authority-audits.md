# ast-grep runtime-authority audits

Molten uses ast-grep as structural evidence for runtime authority seams. Most rules remain inventory-only; the converted local-store adapter rule is blocking within its narrow path scope. The profile does not admit authority, prove replay correctness, certify sealed repro bundles, grant UCAN authorization, prove distributed safety, or establish release readiness.

## Profile

r[impl aspen.ast_grep_runtime_authority_audits.profile] The `runtime-authority` profile covers these scan surfaces:

| Surface | Example scope |
| --- | --- |
| `core-runtime` | `src/runtime/**/*.rs`, `src/node/runtime.rs` |
| `node-control` | `src/node/**/*.rs`, `src/cli/ops/node/**/*.rs` |
| `effect-handlers` | `src/effects/**/*.rs`, `src/resources/**/*.rs` |
| `plugin-host` | `src/plugin/**/*.rs`, `docs/plugin-extension-contracts/**/*.ncl` |
| `sealed-repro` | `src/harness/**/*.rs`, `src/cli/runtime/repro/**/*.rs` |
| `iroh-transport` | `src/iroh/**/*.rs`, `src/node/iroh.rs` |
| `policy-evidence-gates` | `src/evidence/**/*.rs`, `cairn-policy/**/*.ncl` |
| `operator-workflow` | `src/operator/**/*.rs`, `docs/production-*.ncl` |
| `local-store-adapters` | Converted artifact, chunk, retention, dataspace, and exchange adapter pages |
| `test-workspace-shells` | Shared CLI and representative converted unit-test workspace helpers |

## Inventory rules

r[impl aspen.ast_grep_runtime_authority_audits.inventory] The initial rules stay at `hint` severity and are treated as candidate findings only.

| Rule | Category | Posture |
| --- | --- | --- |
| `ambient-filesystem-call` | ambient filesystem | inventory |
| `ambient-process-command` | ambient process | inventory |
| `ambient-network-bind` | ambient network | inventory |
| `ambient-clock-now` | ambient clock | inventory |
| `ambient-random-thread-rng` | ambient randomness | inventory |
| `credential-env-var` | credential access | inventory |
| `plugin-dynamic-load` | plugin loading | inventory |
| `unsafe-block` | unsafe hotspot | inventory |
| `panic-bypass` | panic hotspot | inventory |
| `direct-authority-bypass` | direct authority bypass candidate | inventory |
| `store-ambient-filesystem-call` | ambient child I/O or root reacquisition in converted stores | **blocking** |
| `test-ambient-temp-workspace` | predictable ambient temporary roots or broad prefix cleanup | **blocking** |

r[impl molten.chunk_store.cap_std_regression_gate] The blocking store rule is path-scoped, ignores test fixture trees, and has dedicated positive and negative fixtures. It permits typed root bootstrap/delegation because those shapes do not directly call ambient child APIs. Explicit output materialization remains shell-owned outside the scanned store-page scope.

r[impl molten.testing.cap_std_regression_gate] The test-workspace rule is limited to helpers migrated to the shared `cap-tempfile` shell. It rejects ambient temporary-root construction and stale-prefix deletion while permitting capability-rooted workspace acquisition and explicit selected-artifact export.

Rules live under `tools/ast-grep/runtime-authority/rules/`. Positive and negative fixtures live under `tools/ast-grep/runtime-authority/fixtures/`; fixture coverage is required before any rule can be promoted from inventory to warning or blocking posture.

## Receipt binding

r[impl aspen.ast_grep_runtime_authority_audits.identity] r[impl aspen.ast_grep_runtime_authority_audits.evidence_gates] A runtime-authority audit receipt must bind:

- ast-grep tool version;
- BLAKE3 rule bundle identity;
- BLAKE3 scan scope identity;
- runtime or evidence-gate run identity;
- findings summary and referenced rule ids;
- non-claim labels: not authority admission, not replay proof, not sealed-repro proof, not UCAN authorization proof, not distributed-safety proof, and not release-readiness proof.

A changed rule bundle requires a fresh scan before its finding summary can be reused by an evidence gate.

## Validation

r[verify aspen.ast_grep_runtime_authority_audits.validation] Representative fixture checks:

```sh
ast-grep scan --rule tools/ast-grep/runtime-authority/rules/ambient-filesystem-call.yml \
  --json=compact tools/ast-grep/runtime-authority/fixtures/positive/inventory_candidates.rs
ast-grep scan --rule tools/ast-grep/runtime-authority/rules/ambient-filesystem-call.yml \
  --json=compact tools/ast-grep/runtime-authority/fixtures/negative/allowed_shell_effects.rs

# Must report blocking findings.
ast-grep scan --rule tools/ast-grep/runtime-authority/rules/store-ambient-filesystem-call.yml \
  --json=compact tools/ast-grep/runtime-authority/fixtures/positive/store_ambient_filesystem_calls.rs

# Must report no findings, including the ignored adversarial-test fixture.
ast-grep scan --rule tools/ast-grep/runtime-authority/rules/store-ambient-filesystem-call.yml \
  --json=compact \
  tools/ast-grep/runtime-authority/fixtures/negative/store_capability_shells.rs \
  tools/ast-grep/runtime-authority/fixtures/negative/tests/adversarial_store_setup.rs

# Must report no findings across converted production adapter pages.
ast-grep scan --rule tools/ast-grep/runtime-authority/rules/store-ambient-filesystem-call.yml \
  --json=compact \
  src/artifacts/parts/mod src/chunk/parts/store src/retention/parts/mod \
  src/remote/parts/dataspace src/iroh/parts/exchange

# Must report prohibited predictable-root and broad-cleanup findings.
ast-grep scan --rule tools/ast-grep/runtime-authority/rules/test-ambient-temp-workspace.yml \
  --json=compact \
  tools/ast-grep/runtime-authority/fixtures/positive/test_ambient_temp_workspace.rs

# Must report no findings for the shared capability workspace shape.
ast-grep scan --rule tools/ast-grep/runtime-authority/rules/test-ambient-temp-workspace.yml \
  --json=compact \
  tools/ast-grep/runtime-authority/fixtures/negative/test_capability_workspace.rs
```

Rust unit tests validate the pure profile, fixture-promotion gate, BLAKE3 identity binding, stale-rule-bundle detection, and evidence-only receipt non-claims.
