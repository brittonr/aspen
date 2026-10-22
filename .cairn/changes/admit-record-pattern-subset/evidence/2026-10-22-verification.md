# Admit a bounded record pattern subset: verification

## What changed

- `src/runtime/predicates/parts/mod/p011/body.rs`: the bounded record pattern form. A record pattern declares a record
  label and its declared field positions, and each declared position carries a sub-pattern. The AST form is
  `runtime-pattern-record-v1`. The admitted bounds are sixteen declared positions per record, four nested records, and
  the existing binding byte limit. The positions must increase in Preserves order.
- `src/runtime/predicates/parts/mod/p000/body.rs`: the pattern type carries the new `Record { label, entries }` variant.
  The AST reader, the observe reader, the AST writer, the validation, and the matching handle it. The record match is
  at-least: the candidate's undeclared positions are ignored, a missing or mistyped declared position fails the match,
  and a failed sub-pattern discards the bindings the visit produced.
- `src/runtime/predicates/parts/mod/tests/m000/p007/body.rs`: the record pattern fixtures.
- `src/remote/parts/dataspace/p011/body.rs` and `src/remote/parts/dataspace/p002/body.rs`: the named consumer. The
  two-peer harness's `consumer` actor subscribes to the `service-ready` record shape instead of one exact value.
- `src/remote/parts/dataspace/tests/m000/p004/body.rs`: the consumer fixture that shows a record that carries an extra
  field still reaches the subscriber.
- `docs/architecture.md`: the admitted pattern subset, its bounds, and the routing-only non-claim.

## Validation

- `cargo test -p molten --lib logic::tests`: 32 passed, including the eight record pattern fixtures.
- `cargo test -p molten --lib meshspace`: 26 passed, including the consumer fixture.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings` over the workspace: exit 0.
- `cargo test --workspace` with a fresh `CARGO_TARGET_DIR`: exit 0. The molten library suite reported 1,697
  passed, the core suite 497 passed, the native system-extension integration suite 9 passed, and the doc-test phase
  passed.
- `cargo octet check --artifact-dir target/octet-precommit`: `Status: clean`, 0 findings, 0 warnings, 0 errors.
- `nix run "path:$CAIRN_ROOT#cairn" -- validate --root . --policy
  "$CAIRN_ROOT/cairn-policy/generated/cairn-policy.json" --strict`: exit 0 with no findings, `spec_findings: []` and
  `substance_findings: []`.
- `cargo nextest` cannot run in this checkout: `cargo-nextest` always calls `cargo metadata --all-features`, and that
  call panics in the pinned cargo on the optional `rad://` sources. The panic reproduces on the unmodified tree and is
  unrelated to this change, which touches no manifest. The documented Cargo fallback was used.

## Deferred scope

No array, dictionary, conjunction, or negation pattern is admitted. The record form is the only extension.

## Non-claims

- Pattern matching decides routing only. It grants no authority, computes no identity, feeds no receipt hash, and
  leaves a matched record's canonical value ref unchanged.
- An at-least match accepts a candidate with more structure than the pattern declares. It proves shape acceptance, not
  semantic compatibility with a newer producer.
- The admitted bounds bound the pattern's declared positions and nesting depth. They bound the work a single match
  performs, not the number of subscriptions.
