# ast-grep runtime-authority audits

Molten uses ast-grep as inventory-only structural evidence for runtime authority seams. The profile does not admit authority, prove replay correctness, certify sealed repro bundles, grant UCAN authorization, prove distributed safety, or establish release readiness.

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
```

Rust unit tests validate the pure profile, fixture-promotion gate, BLAKE3 identity binding, stale-rule-bundle detection, and evidence-only receipt non-claims.
