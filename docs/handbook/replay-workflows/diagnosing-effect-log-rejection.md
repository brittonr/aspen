# Diagnosing effect-log rejection
Mode: Troubleshooting

Use this guide when recorded-effect evidence is rejected or a replay result is being mistaken for proof that an effect log is valid. Start by preserving the original recording, consumption observations, expected run/profile refs, and exact error or receipt. Work on separate outputs; do not delete state, renumber an original log, replace an uncertain response with a live call, or retry an external operation to make replay pass.

This is source-checked diagnosis, not an executed incident reproduction. The [hardening contract](../../replay-effect-log-hardening.md) states the intended boundary. The [playback companion](../../technical/foundations/deterministic-playback-contract.md) explains why effect integrity and final-state equality are different questions.

## First distinguish the producing API

`validate_effect_log` consumes explicit `EffectLogEntry` and `ConsumedEffect` slices. Fixture `verify` parses a built-in-style fixture and runs a narrower helper. Harness report validation reconstructs an effect log from observations and checks equality with the report's log. These are separate paths with different diagnostic evidence.

A useful incident header records the entrypoint, input schema, canonical input refs if available, and whether the outcome is a parse error, a validation denial receipt, or a trace comparison denial. Without that classification, the same word “replay” can send investigation to the wrong owner.

## Symptom: no validation receipt was produced

**Discriminating evidence:** Inspect the error boundary. [Input validation](../../../src/deterministic/parts/replay/p006/body.rs) checks expected refs, slice bounds, and each entry's references and kind before constructing a receipt. Each slice is limited to 1024 items. [Kind validation](../../../src/deterministic/parts/replay/p007/body.rs) accepts only nonempty tokens containing lowercase ASCII letters, digits, `-`, or `_`.

**Safe next action:** Ask the producer for the original typed input and serialization context. Distinguish malformed content refs from semantic mismatches. For fixture files, also inspect embedded identity/effect-log hash consistency and top-level output/state bindings in the [fixture parser](../../../src/deterministic/parts/replay/p011/body.rs).

**Stop condition:** Do not manufacture a new hash around edited evidence and present it as the original recording. If the correct producer cannot recover the value, classify the original evidence as unusable for that validation question.

## Symptom: wrong run identity or handler profile

**Discriminating evidence:** The validator checks every recorded entry against `expected_run_identity_ref` and `expected_handler_profile_ref` before sequence diagnostics. An otherwise ordered log can therefore deny immediately for stale context.

**Safe next action:** Compare the intended context with the actual capture context. Use the [freshness contract](../../replay-identity-freshness.md) to distinguish artifact, policy, revocation, runtime, tool, and recorded-input changes. Request evidence for the intended identity rather than relabeling the old entries.

**Stop condition:** A matching executable name or final state is insufficient to continue claiming same-context replay. Preserve the old evidence as historical evidence if its original context remains known.

## Symptom: sequence or duplicate-request rejection

**Discriminating evidence:** Recorded sequence numbering starts at zero and advances contiguously by one. Duplicate sequences are checked as entries are visited; gaps or reordered records can instead trigger the expected-versus-found diagnostic. Duplicate request refs are a separate later check.

**Safe next action:** Compare the stored ordered sequence with the capture/export boundary. Look for a truncated prefix, duplicate export, or merged recordings. Request the complete original stream if available. Do not sort or renumber entries in place: sequence is part of what is being validated.

**Stop condition:** If the producer cannot establish original ordering, stop treating this as replayable evidence. The validator's duplicate-request rule does not authorize suppressing a real repeated operation outside the replay model.

## Symptom: response or boundary mismatch

**Discriminating evidence:** For a consumption observation whose sequence matches an entry, checks occur in order: kind, request, response, boundary. An equal response ref cannot repair a wrong request or boundary.

**Worked failure case:** The checked-in [response-mismatch test](../../../src/deterministic/parts/replay/tests/m000/p003/body.rs) supplies one sequence-zero entry and one consumption observation, with different response refs. It expects a denial and a response-mismatch diagnostic. This test demonstrates the relevant supplied-input relationship; it is not newly executed evidence and does not prove adapter instrumentation.

**Safe next action:** Retain both sides and identify which component reported consumption. Ask whether request identity, boundary association, or response selection changed. Trace the first differing binding before investigating later state hashes.

**Stop condition:** Do not substitute the recorded response into an observation merely to make evidence agree. Correct the producer or capture a new, separately identified run.

## Symptom: missing, unused, or live-fallback effect

**Discriminating evidence:** Unconsumed recorded entries are checked before missing recordings; live fallback is checked after both. A missing-entry diagnostic can therefore mask a simultaneously reported fallback. Only the first semantic diagnostic is retained.

**Safe next action:** Establish whether the full recording and all consumption facts were supplied. If a fallback occurred, treat it as a different execution context, not a replay repair. Do not issue another external request to recover an old response.

**Stop condition:** Unknown external-effect completion requires effect-specific recovery evidence, not unconditional retry. Set-based consumption coverage is not an exactly-once guarantee and does not independently reject every duplicate consumption pattern.

## Symptom: fixture verification appears stronger than its evidence

The governing hardening page says fixture verification invokes effect validation before downstream comparison. The inspected [helper](../../../src/deterministic/parts/replay/p006/body.rs) does call that validator, but constructs one matching clock entry and consumption observation from journal refs, uses a default profile, and sets live fallback false. The [caller](../../../src/deterministic/parts/replay/p001/body.rs) propagates errors but does not branch on a returned denial decision.

This is a source-review discrepancy in enforcement scope, not a reproduced bug. A fixture verify receipt should not be reused as proof that arbitrary embedded effect-log entries were independently validated. Stop that stronger claim; obtain explicit entry/consumption validation evidence from the actual producer. Also keep harness validation separate: its [report validator](../../../src/harness/replay.rs) explicitly compares observation-derived effects to the report log.

## Sources

- [Handbook](../README.md)
- [Effect-log hardening contract](../../replay-effect-log-hardening.md)
- [Deterministic playback companion](../../technical/foundations/deterministic-playback-contract.md)
- [Validation precedence](../../../src/deterministic/parts/replay/p006/body.rs)
- [Sequence and binding diagnostics](../../../src/deterministic/parts/replay/p007/body.rs)
- [Effect validation regression cases](../../../src/deterministic/parts/replay/tests/m000/p003/body.rs)
