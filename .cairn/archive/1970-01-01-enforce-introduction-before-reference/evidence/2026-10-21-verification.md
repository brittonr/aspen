# Enforce introduction before reference: verification

## What changed

- `src/remote/parts/dataspace/p010/body.rs`: the reference introduction rule. `IntroducedReferences::derive`
  derives the introduced set for a session from the session's bootstrap references and the references its live
  assertions name, so withdrawing the last assertion that introduced a reference withdraws the introduction.
  `unknown` reports the carried references the peer does not know yet, and `refuse_unintroduced_references`
  is the sender-side refusal.
- `src/remote/parts/dataspace/p006/body.rs`: the session carries its bootstrap references, and
  `admit_and_apply_delivered_envelope_for_session` denies a delivery that carries an unintroduced reference
  before it is applied. The denial names the unknown reference and leaves no partial state.
- `src/remote/parts/dataspace/tests/m000/p005/body.rs`: the introduction fixtures.

The peer-visible surface is unchanged. The denial is recorded as evidence, not answered with a protocol-level
error reply.

## Validation

- `cargo test -p molten --lib meshspace`: the introduction fixtures pass and the existing remote dataspace
  baseline keeps passing.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings` over the workspace: exit 0.
- `cargo test --workspace` with a fresh `CARGO_TARGET_DIR`: exit 0. The molten library suite reported 1,685
  passed, the core suite 497 passed, the native system-extension integration suite 9 passed, and the doc-test phase
  passed.
- `cargo octet check --artifact-dir target/octet-precommit`: `Status: clean`, 0 findings, 0 warnings, 0 errors, with
  `cargo-octet 0.1.0` on `nightly-2026-03-21`.
- `nix run "path:$CAIRN_ROOT#cairn" -- validate --root . --policy
  "$CAIRN_ROOT/cairn-policy/generated/cairn-policy.json" --strict`: exit 0 with no findings, `spec_findings: []` and
  `substance_findings: []`.
- `cargo nextest` cannot run in this checkout: `cargo-nextest` always calls `cargo metadata --all-features`, and that
  call panics in the pinned cargo on the optional `rad://` sources. The panic reproduces on the unmodified tree and is
  unrelated to this change, which touches no manifest. The documented Cargo fallback was used.

## Deferred scope

The sender-side refusal is delivered as a rule, not wired into the envelope build sites. The build sites
do not hold the receiver's session view: the operator's remote-envelope command knows the target peer and
topic but not the session generation that fixes the owner's introduced set. The receiver-side enforcement is
the enforcing boundary, and the sender-side wiring stays open until the sender's session view exists.

## Non-claims

- The rule decides reference admission only. It grants no authority, no delivery guarantee, and no claim that a
  reference is current.
- The introduced set is derived from the session's live assertions and its bootstrap references. A session
  that closes keeps no facts, so it introduces nothing.
