# Reviewing a counterexample for reuse
Mode: Review checklist

A reusable counterexample is more than a denial string. It gives a future reviewer the inputs, context, and bounded failing relationship needed to interpret the failure without repeating uncertain external effects. Use this checklist before promoting a local replay discrepancy into a regression fixture, shared investigation bundle, or evidence attached to another subject.

This review is source-backed, not runtime-verified. No command or test was executed for this guide. The [deterministic playback companion](../../technical/foundations/deterministic-playback-contract.md) supplies the theory; the questions below specify concrete acceptance evidence and reasons to withhold a stronger claim.

## 1. Is the claimed boundary explicit?

- [ ] Does the submission name its producer API and input family: deterministic fixture, summary comparison, effect-log validation, harness report, or world replay capsule?
- [ ] Does it distinguish “two supplied summaries differ” from “an execution replayed differently” and from “recorded effects were rejected”?
- [ ] Does it name expected and actual roles and explain why the expected recording is the reference for this question?
- [ ] Is execution status explicit? Source inspection, a checked-in test, and an observed run receipt are different evidence classes.

**Required evidence:** original input references or files, producer/source revision information, and the exact receipt or error. A screenshot of a denial without its bound inputs is insufficient for reusable evidence. If the reviewer only has source reasoning, accept it as a source-review observation, not a reproduced defect.

## 2. Can another reviewer recover the same bounded inputs?

- [ ] Are both fixture values retained rather than just their summary hashes?
- [ ] Are identity/effect-log embedded values consistent with their declared refs under canonical hashing?
- [ ] Are top-level output and final-state refs bound to the journal selected by the fixture parser?
- [ ] If the claim is multi-turn, does the producer materialize every required boundary rather than relying on the fixture parser's first-journal selection?
- [ ] For partial manifest-backed inspection, is there actual range-read evidence, not merely the prefix receipt's `range-receipt-required` marker?

The [fixture parser](../../../src/deterministic/parts/replay/p011/body.rs) and [summary API](../../../src/deterministic/parts/replay/p013/body.rs) define different scopes. An appended second journal is not proof that the fixture comparison consumed it. Likewise, the summary validator checks bounded shape and refs, not chronological completeness or contiguous event positions. Require producer evidence for ordering.

## 3. Is the divergence artifact really available?

- [ ] Does the comparison receipt retain expected/actual summary refs and the first-divergence ref?
- [ ] If the review quotes a field path or actor label, is the corresponding canonical path record available?
- [ ] Does the quoted path come from the correct comparison direction and boundary order?
- [ ] Are missing sides represented as `none`, without a fabricated content ref?

The [core result](../../../src/deterministic/parts/replay/p013/body.rs) returns `first_divergence` separately. The [fixture shell](../../../src/cli/test/replayfixture/ops.rs) writes the receipt value, not that separate record. An explain receipt links the comparison and divergence refs but does not resolve the path. Withhold claims about an unavailable path payload; retain the input pair so an integration can recover and preserve the core result.

## 4. Does reuse preserve the intended identity?

- [ ] Have artifact, dependency closure, initial state, schemas, policies, capabilities, revocations, handler profile, seed-or-effect-log, runtimes, tools, and replay profile been considered?
- [ ] If reuse targets another subject, is expected identity supplied independently of the old evidence identity?
- [ ] Does any freshness receipt bind that subject and evidence, rather than merely showing that an old record matches itself?
- [ ] Has handler-profile equality been established separately when needed? The inspected comparison selector has no standalone profile-mismatch branch.

**Required evidence:** a complete intended identity and the evidence identity, or a [freshness result](../../../src/deterministic/parts/replay/p009/body.rs) over those supplied values. A freshness pass means applicable evidence, not current admission, policy approval, release eligibility, or authority to execute.

## 5. Is effect integrity supported independently?

- [ ] Are recorded entries and consumption observations available from the actual effect boundary?
- [ ] Were shape failures separated from semantic denial receipts?
- [ ] Is the first diagnostic treated as a localization result, not an exhaustive list of all problems?
- [ ] Does the review avoid inferring exactly-once consumption or external delivery from sequence coverage?
- [ ] Are live fallback and uncertain effect completion treated as stop conditions rather than reasons for unconditional retry?

The [effect validator](../../../src/deterministic/parts/replay/p006/body.rs) compares supplied observations. Its fixture helper synthesizes a matching entry/observation from journal refs; that is not independent instrumentation of arbitrary embedded logs. Record this limitation when reusing a fixture verify receipt.

## 6. Worked acceptance decision

Consider the checked-in `ChangedEffectResponse` case in the [comparison tests](../../../src/deterministic/parts/replay/tests/m000/p005/body.rs). A sound reusable example retains baseline and changed fixture values, identifies the built-in response change, and expects the first semantic divergence at `turns[0].effect-response-ref`. It describes a controlled single-turn comparison counterexample, not nondeterminism in a live clock adapter.

Accept it for that bounded regression purpose when inputs and path evidence are available. Do not accept a claim that `tamper` preserved an arbitrary supplied run: the shell parses the input and generates a selected built-in variant. Do not accept the same example as release evidence for an unrelated artifact without the separate subject/freshness checks. These decisions preserve useful evidence without upgrading its authority.

## Sources

- [Handbook](../README.md)
- [Replay identity freshness contract](../../replay-identity-freshness.md)
- [Multi-turn compare and explain contract](../../replay-multiturn-explain.md)
- [Deterministic playback companion](../../technical/foundations/deterministic-playback-contract.md)
- [Fixture parser](../../../src/deterministic/parts/replay/p011/body.rs)
- [First-divergence selection](../../../src/deterministic/parts/replay/p014/body.rs)
- [Effect-log validation cases](../../../src/deterministic/parts/replay/tests/m000/p003/body.rs)
