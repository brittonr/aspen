# Comparing two recorded runs
Mode: How-to

## Goal and prerequisites

Produce reviewable evidence about two compatible recorded fixture values without replacing the expected run or performing external effects. You need both original Preserves files, their provenance, a compatible `molten` binary, and a new output directory. Preserve the distinction between an expected reference run and an actual candidate; swapping them changes the evidence even if both comparisons deny.

This procedure is source-checked and not executed. It does not claim that arbitrary application recordings can be passed to the fixture CLI. The [multi-turn contract](../../replay-multiturn-explain.md) describes comparison over materialized summaries; the current fixture shell is a narrower adapter to that core.

## 1. Decide which recorded object you actually have

Use the object family, not its filename, to choose the path:

- A `deterministic-fixture-record-v1` value belongs to `replay-fixture compare`.
- A harness report belongs to the harness replay path. [Harness replay](../../../src/harness/replay.rs) parses its embedded suite, runs it with its effect log, and compares report boundaries. It is not the same as comparing two fixture records.
- Already materialized `ReplayTraceSummary` values can be supplied to the Rust core `compare_replay_summaries`. The inspected fixture CLI does not accept a summary file as a separate input format.
- World transition traces and capsules have a separate [exact replay workflow](../../technical/world-effects/exact-replay-and-bounded-capsules.md); do not relabel them as fixture records.

Stop if the recording's producer or schema is unknown. A file that parses as Preserves is not thereby a valid fixture, a faithful execution transcript, or a source of authority.

## 2. Establish the comparison question before running it

For a regression question, retain the intended run identity and the reason each input was selected. A changed artifact, policy, initial state, or handler profile can explain a difference without demonstrating nondeterminism. For evidence reuse against a new subject, use the separate [freshness contract](../../replay-identity-freshness.md): comparison success does not itself establish that evidence matches today's intended identity.

Do not use `verify` as a substitute for pairwise comparison. [Fixture verification](../../../src/deterministic/parts/replay/p001/body.rs) constructs the built-in baseline and compares the supplied fixture against it. `compare` instead accepts an explicit expected/actual pair. Two identical non-baseline fixtures can match each other while not matching the built-in baseline; those are different questions, not contradictory results.

## 3. Compare into a new output location

Set `EXPECTED_REPLAY` and `ACTUAL_REPLAY` to real retained fixture files and `REPLAY_REVIEW` to a new directory. The guards are local shell safety checks, not admission checks or protection against concurrent writers. The shell writer can overwrite an existing file; use an isolated path and do not share it with another process.

The command prefix is declared in the [parent enum](../../../src/main/root/parts/command/p000/body.rs), wired through [main's aliases](../../../src/main.rs), and specified by the [replay command enum](../../../src/cli/test/replayfixture/command.rs). The [operations](../../../src/cli/test/replayfixture/ops.rs) and [I/O helper](../../../src/cli/test/replayfixture/io.rs) establish artifact/output behavior. Source-checked, not executed:

```sh
: "${EXPECTED_REPLAY:?Set the expected fixture path}"
: "${ACTUAL_REPLAY:?Set the actual fixture path}"
: "${REPLAY_REVIEW:?Set a new isolated output directory}"
if [ -f "$EXPECTED_REPLAY" ] && [ -f "$ACTUAL_REPLAY" ] && [ ! -e "$REPLAY_REVIEW" ]; then
  molten test replay-fixture compare "$EXPECTED_REPLAY" "$ACTUAL_REPLAY" \
    --receipt-out "$REPLAY_REVIEW/comparison.preserves" &&
  molten test replay-fixture explain "$REPLAY_REVIEW/comparison.preserves" \
    --out "$REPLAY_REVIEW/explanation.preserves"
fi
```

A completed shell operation is not synonymous with `decision=pass`. The operation writes the comparison result and returns successfully even when that result is a denial. Review the canonical receipt decision rather than treating shell chaining as a pass gate.

## 4. Read the result at its actual granularity

The comparison receipt binds expected and actual summary references, decision, first-divergence reference, redaction status, and checks. Comparison order is identity, ordered boundary vector, effect-log refs, output refs, final state, then aggregate turn refs. The earliest difference in that order wins.

Worked case: the built-in response variant changes `logical-time:42` to `logical-time:43`. The [existing tests](../../../src/deterministic/parts/replay/tests/m000/p005/body.rs) expect an `effect-response` difference at `turns[0].effect-response-ref`, before the downstream output differences. This is a useful localization result; it is not evidence that a live clock was consulted.

If the CLI receipt names a divergence you cannot inspect, do not invent its payload. The core returns `first_divergence` separately, but the shell writes only `receipt.value`. `explain` links existing references; it is not a resolver for the omitted path record. Preserve the inputs for a core integration that retains that returned value.

## 5. Decide whether the evidence is sufficient

For the built-in single-turn exercise, retain the original pair and both generated receipts. For a genuine multi-turn claim, stop before acceptance unless the producer supplies every required ordered boundary through the summary API: the current fixture parser selects only the first journal. For live-effect integrity, obtain explicit consumed-effect observations; comparison is not a live-effect monitor. For current deployment or release eligibility, obtain the separate authority and freshness evidence. A matching replay receipt grants none of those permissions.

## Sources

- [Handbook](../README.md)
- [Deterministic playback companion](../../technical/foundations/deterministic-playback-contract.md)
- [Multi-turn comparison contract](../../replay-multiturn-explain.md)
- [Fixture parser](../../../src/deterministic/parts/replay/p011/body.rs)
- [Summary and comparison API](../../../src/deterministic/parts/replay/p013/body.rs)
- [Explain and comparison ordering](../../../src/deterministic/parts/replay/p014/body.rs)
