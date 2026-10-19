# Resource lifecycle count preflight verification

## Scope and failing-before reproduction

The isolated clean worktree `change/preflight-resource-lifecycle-20260929` starts at canonical Aspen `81baf80cb667a22277be22c140d34829e5c7da7f`. In this source layout the public `evaluate_placement` and `validate_reconcile_completion` implementations live in `src/resources/parts/mod/p015/body.rs` and `p016/body.rs`; their sibling 64/128 caps are declared in `p007/body.rs` and `p008/body.rs`. The detached composition has integrated implementations in `p007` and `p008`, so the same checks were applied to the actual functions there. Search for both helpers across the owner repository found only unit-test callsites; no production CLI path exists to exercise.

Before editing production source, `CARGO_TARGET_DIR=/home/brittonr/.cargo-target/aspen-resource-lifecycle-owner-20260929 nix develop --command cargo test -p molten --lib resource_lifecycle_preflights_ -- --test-threads=1` ran both new focused regressions and failed both one-past-bound assertions. The 65-constraint placement request returned `required constraint not satisfied: zone In ["east"]` rather than a count denial; 129 required effect intents formatted a stale-generation diagnostic and every missing effect rather than one bounded count denial. The same tests had already passed their exact 64/128 positive assertions and admitted-invalid precedence assertions before reaching those failures.

## Corrected public behavior

`evaluate_placement` now checks its caller constraint count against existing `MAX_CONSTRAINTS = 64` before allocating or formatting diagnostics or extracting/duplicating target taints. `validate_reconcile_completion` checks required-intent count against existing `MAX_EFFECT_INTENTS = 128` before per-required-effect formatting. An over-limit request returns a single count diagnostic with the original `PlacementDecision` or `ReconcileCompletionDecision` shape; admitted input retains the prior required-constraint and stale-generation-before-effect diagnostic precedence.

After the source fix, the same focused owner filter passed **2/2**, the owner `supplies::tests::` library filter passed **47/47**, and the composed changed-path filter passed **2/2**. A separately compiled disposable Rust package depending on the public `molten::supplies` facade checked pass at exactly 64 preferred constraints, denial at 65, pass with 128 receipt-backed intents, denial at 129, and admitted-input errors on both helpers. Its output was `external resource lifecycle: placement 64 pass/65 deny; completion 128 pass/129 deny; admitted errors preserved`; its temporary source was removed.

The distinct `explicit_target_properties` input still has no approved count or byte-size contract. This owner did not cap it, alter public signatures/decisions for admitted inputs, or change policy, waiver, pin, accepted spec or the unrelated full strict-ci gate.

## Scoped checks and residual policy boundary

- Owner `supplies::tests::` passed **47/47**; composed `supplies::tests::` passed **49/49**, including two separate prior deletion-gate owner tests, without overwriting that owner's pre-existing composed test content.
- `nix develop --command cargo fmt --package molten --check` exited 0 in both worktrees.
- Owner independent-target `cargo clippy -p molten --all-targets -- -D warnings` and composition independent-target `cargo clippy -p molten --lib -- -D warnings` passed.
- Owner independent, nonincremental `cargo octet check --artifact-dir /tmp/molten-resource-lifecycle-owner-lib-20260929 -- -p molten --lib`: `status.json` reports `warning-only`, **835** warnings, zero errors/autofixable.
- Composed independent, nonincremental `cargo octet check --artifact-dir
  /tmp/molten-resource-lifecycle-composition-lib-20260929 -- -p molten
  --lib`: `status.json` reports `warning-only`, **1,493** warnings,
  zero errors/autofixable. Its static `unbounded_collection_growth`
  findings still point at `src/resources/parts/mod/p007/body.rs:300` and
  `p008/body.rs:339` despite runtime entry guards bounding iteration
  by 64 and 128. The owner source layout is split and its scoped
  summary has no `src/resources` rows. Neither warning-only library
  profile proves the full-catalog strict gate or clears these static rows.
- Owner proposal/design Cairn gates each returned advisory `valid: true`
  without issues. `nix develop --command cairn validate --root . --policy
  cairn-policy/generated/cairn-policy.json --strict` returned empty issue
  lists and `valid: true` for **53** isolated and **65** composed change
  packages. These are structural validations, not release acceptance.
