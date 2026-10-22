# Record turn causality in traces: verification

## What changed

- `src/runtime/turn/causality.rs`: the closed cause vocabulary `RuntimeTurnCause` with `prior-turn`,
  `cleanup`, `linked-task-release`, `periodic-activation`, `delay`, and `external`. The cause has a canonical
  record, a decoding step that refuses an absent, unknown, or malformed cause, and a bounded external
  description. The new `RuntimeEvent::TurnCommitted` record carries the actor, the cause, and the refs of the
  turn's events, so the cause is inside the hashed identity. The chronicle keeps the event records.
- `src/runtime/turn/mod.rs`: the pending turn carries the actor and the cause.
- `src/runtime/dataspace/parts/state/p001/body.rs`: `apply_step_with_cause` and `begin_turn_with_cause` take
  the cause from the transition input, and the commit path appends the committed turn record. The plain
  `apply_step` keeps the external cause it always had.
- `src/runtime/dataspace/mod.rs` and the two cleanup paths: the scope cleanup record names `cleanup` as its
  cause.
- `src/runtime/dataspace/parts/syndicate/p001/body.rs`: the parity receipt compares the semantic actions. The
  committed turn record is Molten's causal annotation and the reference surface has no counterpart, so the
  parity skips it while the trace evidence keeps binding it.
- `src/harness/parts/schema/p002/body.rs` and `src/service/parts/runtime/p003/body.rs`: the new event is
  rendered and labelled.

The recorded trace fixtures that asserted an exact event list were re-recorded: the dataspace turn tests, the
predicate turn tests, and the facet-stop ordering test now expect the committed turn record after the turn's
events.

## Validation

- `cargo test -p molten --lib`: 1,680 passed, 0 failed, including the new cause fixtures: a dependency-driven
  turn names the releasing turn, a cleanup turn names `cleanup`, a delayed turn names its causing turn and
  amount, the cause vocabulary refuses an unknown, absent, and malformed cause, and a replay whose cause differs
  compares as divergent while the same cause replays to the same turn record.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings` over the workspace: exit 0.
- `cargo test --workspace` with a fresh `CARGO_TARGET_DIR`: exit 0. The molten library suite reported 1,680
  passed, the core suite 492 passed, the native system-extension integration suite 9 passed, and the doc-test phase
  passed.
- `cargo octet check --artifact-dir target/octet-precommit`: `Status: clean`, 0 findings, 0 warnings, 0 errors, with
  `cargo-octet 0.1.0` on `nightly-2026-03-21`. The first runs reported findings in the new code: 2 on the first pass
  (the chronicle grew past the file-length limit) and 6 on the second (file length, path-segment repetition, and
  non-trait imports). The cause vocabulary moved into its own module, the module name no longer repeats an item
  word, and the owner paths are used at the use site; the re-run is clean.
- `nix run "path:$CAIRN_ROOT#cairn" -- validate --root . --policy
  "$CAIRN_ROOT/cairn-policy/generated/cairn-policy.json" --strict`: exit 0 with no findings, `spec_findings: []` and
  `substance_findings: []`.
- `cargo nextest` cannot run in this checkout: `cargo-nextest` always calls `cargo metadata --all-features`, and that
  call panics in the pinned cargo on the optional `rad://` sources. The panic reproduces on the unmodified tree and is
  unrelated to this change, which touches no manifest. The documented Cargo fallback was used.

## Non-claims

- A trace proves the recorded causal order only. It is not a complete account of what happened, and it is not
  sampled or aggregated.
- A cause is never derived from a wall clock, and the vocabulary is closed: an unknown cause is refused rather
  than recorded as free text. `external` is the escape hatch for inputs outside the runtime model.
- The change adds no authority, no receipt semantics, and no completeness claim.
