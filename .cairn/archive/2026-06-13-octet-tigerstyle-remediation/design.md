## Context

Early Octet summaries highlighted blockers to strict mode: high-volume lint families, oversized shell files, high-arity helpers, unbounded collection growth, raw/stringly refs, and critical caveats such as panic/unwrap/time/resource-shape findings. Current strict evidence is configuration-clean with disabled lint families documented in `dylint.toml` and `docs/octet-tigerstyle-remediation.md`.

Some lints are style/noise in early scaffolding. Others are direct Tiger Style and fail-close blockers: long functions hide control flow, high-arity builders produce mismatched evidence fields, unbounded collections undermine deterministic resource policy, raw strings blur authority/ref identity, and panic/unwrap/time caveats can make evidence unreplayable. This archive distinguishes completed gate/evidence remediation from future source-remediated-zero module splits.

## Remediation principles

1. **No suppression-first cleanup.** Remove or structurally fix warnings before adding review receipts.
2. **Critical surfaces first.** Prioritize runtime admission, harness/gate validation, job execution, node startup, ledger/evidence, adapter boundaries, and redaction/export paths.
3. **Functional core, imperative shell.** Move parsing, evaluation, and receipt decisions into pure helpers; keep filesystem/process/time shell code thin and receipt-bound.
4. **Input structs for builders.** Receipt/value constructors with many arguments should take validated input structs to prevent field-order bugs.
5. **Bounded resources.** Every data-dependent loop/collection on evidence paths should have explicit limits, budget checks, or proofs that the input is bounded by a prior artifact.
6. **Typed identities.** Public or cross-boundary functions should not accept interchangeable raw strings for artifact/schema/policy/receipt/capability/secret/effect refs.
7. **Receipt-backed failures.** Critical gate paths should return structured denials instead of panic/unwrap/expect.

## Initial hotspot plan

### `src/main.rs`

- Future source-remediated-zero work should extract command groups into module-specific dispatch files under a CLI shell namespace.
- Keep Clap-only parsing at the edge; convert CLI strings into typed refs before calling core modules.
- Replace long match arms with command handler functions under bounded sizes where touched.
- Ensure commands that request artifact output emit canonical failure artifacts rather than relying on stderr.

### `src/job_dag.rs`

- Future source-remediated-zero work should split canonical DTOs, parsing, sync, admission, execution, and tests into submodules without changing canonical refs.
- Replace high-arity request/receipt builders with input structs where touched.
- Add explicit collection bounds for stage lists, selected stage sets, diagnostics, receipt refs, and artifact closure traversal.
- Remove sentinel fallback patterns in admission/execution validation.

### `src/node_runtime.rs`

- Convert `node_config_value`, `node_startup_receipt_value`, and adapter receipt builders to input structs.
- Promote deterministic adapter order into a public/pure helper with bounded adapter count and duplicate detection.
- Replace stringly adapter/profile/receipt refs with typed local structs at public boundaries.
- Add tests proving denied startup emits canonical receipts without panic or ambient state.

## Measurement

Each remediation slice should report:

- before/after Octet finding counts by lint and path, or configuration-clean caveats when disabled lint families are in use;
- strict/quarantine gate receipt refs;
- remediation-plan refs;
- object corpus/fingerprint refs for changed critical paths;
- focused cargo/clippy/tests/Cairn validation;
- whether warning burn-down target was met or deferred as future source-remediated-zero work.

## Non-goals

- Do not rewrite unrelated stable modules only to satisfy low-value style lints.
- Do not weaken tests or skip validation to reduce warning count.
- Do not treat formatting/import churn as sufficient for strict fail-close; critical behavioral caveats must be removed or reviewed.
