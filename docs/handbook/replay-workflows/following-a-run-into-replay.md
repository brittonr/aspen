# Following a run into replay
Mode: Walkthrough

This walkthrough follows the checked-in `ChangedEffectResponse` fixture construction from inputs to comparison evidence. It is a source walkthrough, not a recording of a live application. The useful outcome is knowing which artifact to retain at each boundary and where the available shell stops. Commands below are source-checked, not executed for this guide; no runtime verification has been performed for this batch.

The [deterministic playback companion](../../technical/foundations/deterministic-playback-contract.md) explains why playback is conditional. Here the concrete path is `record_fixture_value` → `run_parts` → fixture serialization → fixture parsing → summary comparison. The implementation is assembled by [replay.rs](../../../src/deterministic/replay.rs), so the included parts, not the short module facade, contain the behavior.

## 1. Identify the actual input

The [fixture choices](../../../src/deterministic/parts/replay/p003/body.rs) define one baseline turn. Its input message is `message:root-to-helper`, its request payload is `logical-now:turn-0001`, and its recorded response payload is `logical-time:42`. The actor is `actor:helper`; the journal names `turn:0001`. These are fixture strings, not evidence that an adapter read a clock.

`ChangedEffectResponse` selects `logical-time:43` instead. It does not alter the input message or request choice. That makes this a useful counterexample: later output and state references can change because their canonical records depend on the response, while the earlier request boundary remains equal.

The observable source boundary is the `RunChoices` value passed into hash construction. Do not describe this step as capturing arbitrary process execution. The `record` subcommand has no application, suite, or report input; it generates this built-in fixture.

## 2. Follow canonical dependencies

`run_parts` constructs an identity value, hashes it, then constructs effect, output, and state references. The response record binds the request reference and labels its source `recorded-effect-log`. The policy-decision record binds that response. Action, receipt, output, and after-state records extend the dependency chain.

The [fixture packager](../../../src/deterministic/parts/replay/p001/body.rs) embeds identity and effect-log values beside their references, a sequence containing the journal, and output/final-state references. Identity is canonical Preserves plus BLAKE3, not Rust memory layout or the exact whitespace of rendered text. Preserve the whole fixture, not a copied terminal hash alone.

## 3. Materialize an isolated example

Prerequisite: a repository-compatible `molten` binary is already available. Choose a new, non-existing output directory in `REPLAY_DEMO`; the guard below avoids intentionally reusing an existing path. The writer creates parents and writes files, so it is not an archival no-overwrite API.

Spelling comes from the [parent command declarations](../../../src/main/root/parts/command/p000/body.rs), [replay command declarations](../../../src/cli/test/replayfixture/command.rs), and [shell operations](../../../src/cli/test/replayfixture/ops.rs); directory creation is in [shell I/O](../../../src/cli/test/replayfixture/io.rs). Source-checked example, not executed:

```sh
: "${REPLAY_DEMO:?Set a new isolated output directory}"
if [ ! -e "$REPLAY_DEMO" ]; then
  molten test replay-fixture record --out "$REPLAY_DEMO/baseline.preserves" &&
  molten test replay-fixture tamper "$REPLAY_DEMO/baseline.preserves" \
    --kind effect-response --out "$REPLAY_DEMO/changed.preserves" &&
  molten test replay-fixture compare "$REPLAY_DEMO/baseline.preserves" \
    "$REPLAY_DEMO/changed.preserves" --receipt-out "$REPLAY_DEMO/comparison.preserves" &&
  molten test replay-fixture explain "$REPLAY_DEMO/comparison.preserves" \
    --out "$REPLAY_DEMO/explanation.preserves"
fi
```

A notable boundary: `tamper` parses the supplied file, then constructs the selected built-in variant. It is not a general mutation editor that preserves arbitrary input content. Use this recipe only for the named fixture exercise.

## 4. Observe comparison meaning, not imagined output

The [parser](../../../src/deterministic/parts/replay/p011/body.rs) checks embedded identity/effect-log hashes and the output/final-state bindings to the selected journal. The [summary adapter](../../../src/deterministic/parts/replay/p013/body.rs) creates seven ordered semantic boundaries for the first turn. Comparison checks identity first, then those boundaries before downstream aggregates.

The checked-in [comparison test](../../../src/deterministic/parts/replay/tests/m000/p005/body.rs) expects this variant to deny at `turns[0].effect-response-ref`. Its event index is 3. This describes the source-backed expected result, not newly observed terminal output. It is a first differing comparison boundary, not proof of the ultimate causal defect in an application.

## 5. Preserve the boundary the CLI does not export

`ReplayComparisonReceipt` returns a divergence value and its reference in memory. The CLI writes only the comparison receipt value, which contains the divergence reference, not the full path record. `explain` creates another canonical receipt linking the comparison and divergence references; it does not fetch or reconstruct the missing record.

Keep both fixtures and the comparison receipt. If a downstream reviewer needs the complete path artifact, an integration using the core result must retain `first_divergence`; do not claim the shown shell already exports it. Likewise, this fixture adapter selects the first journal, although the core summary API accepts vectors. This walkthrough establishes a one-turn fixture path, not arbitrary multi-turn capture, live replay admission, or production readiness.

## Sources

- [Handbook](../README.md)
- [Multi-turn compare and explain contract](../../replay-multiturn-explain.md)
- [Deterministic playback companion](../../technical/foundations/deterministic-playback-contract.md)
- [Fixture construction](../../../src/deterministic/parts/replay/p003/body.rs)
- [Comparison implementation](../../../src/deterministic/parts/replay/p013/body.rs)
- [Comparison and explain tests](../../../src/deterministic/parts/replay/tests/m000/p005/body.rs)
