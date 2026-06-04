## Context

Octet summary highlights the immediate blockers to strict mode:

- full workspace: `3763` warnings, `142` autofixable;
- lib-only: `1586` warnings, `56` autofixable;
- high-volume lint families: `non_trait_imports`, `path_segment_repetition`, `unbounded_collection_growth`, `too_many_parameters`, `function_length`, `bool_naming`, `no_unwrap`, `excessive_file_length`, `ambient_clock`, `unbounded_loop`, `no_panic`;
- focused paths: `src/job_dag.rs`, `src/main.rs`, and `src/node_runtime.rs` have warning clusters relevant to current development.

Some lints are style/noise in early scaffolding. Others are direct Tiger Style and fail-close blockers: long functions hide control flow, high-arity builders produce mismatched evidence fields, unbounded collections undermine deterministic resource policy, raw strings blur authority/ref identity, and panic/unwrap/time caveats can make evidence unreplayable.

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

- Extract command groups into module-specific dispatch files under a CLI shell namespace.
- Keep Clap-only parsing at the edge; convert CLI strings into typed refs before calling core modules.
- Replace long match arms with command handler functions under 70 lines.
- Ensure commands that request artifact output emit canonical failure artifacts rather than relying on stderr.

### `src/job_dag.rs`

- Split canonical DTOs, parsing, sync, admission, execution, and tests into submodules.
- Replace high-arity request/receipt builders with input structs.
- Add explicit collection bounds for stage lists, selected stage sets, diagnostics, receipt refs, and artifact closure traversal.
- Remove sentinel fallback patterns in admission/execution validation.

### `src/node_runtime.rs`

- Convert `node_config_value`, `node_startup_receipt_value`, and adapter receipt builders to input structs.
- Promote deterministic adapter order into a public/pure helper with bounded adapter count and duplicate detection.
- Replace stringly adapter/profile/receipt refs with typed local structs at public boundaries.
- Add tests proving denied startup emits canonical receipts without panic or ambient state.

## Measurement

Each remediation slice should report:

- before/after Octet finding counts by lint and path;
- strict/quarantine gate receipt refs;
- object corpus/fingerprint refs for changed critical paths;
- focused cargo/clippy/tests/Cairn validation;
- whether warning burn-down target was met.

## Non-goals

- Do not rewrite unrelated stable modules only to satisfy low-value style lints.
- Do not weaken tests or skip validation to reduce warning count.
- Do not treat formatting/import churn as sufficient for strict fail-close; critical behavioral caveats must be removed or reviewed.
