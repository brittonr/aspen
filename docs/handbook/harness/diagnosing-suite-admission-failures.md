# Diagnosing suite admission failures

Mode: Troubleshooting

Start by preserving the original suite and any generated artifact. Do not broaden grants, remove policy, inflate budgets, or edit report references merely to obtain a green result. Those changes alter the question being tested. The paths below are source-checked diagnostic guidance, not reproduced failures or commands executed for this document.

## First split: failed run or correctly denied operation?

The harness has at least three relevant outcomes: input rejected before a suite can execute, execution/report construction failed, or execution completed with a denied step. A valid denial trace can live inside a `harness-report-v1` whose top-level status is `pass`. Conversely, the path requested with `--report-out` can contain `harness-failure-v1` after a handled run failure.

Inspect the artifact's record label, failure phase/kind if present, and command exit status. A filename ending in “report” does not determine its type. The [CLI implementation](../../../src/cli/test/harness.rs) maps initial read/parse errors to preflight; after suite parsing it classifies `InvalidHarness` as preflight and divergence as execute. That phase is useful localization, not proof that no earlier modeled work occurred on every error path.

## Symptom: the text parses, but execution refuses missing evidence

**Discriminating evidence:** Messages identify a missing explicit actor registry, capability fixture, or budget fixture. The parser can infer actors and instantiate defaults, while `prepare_suite_run` rejects the corresponding non-explicit markers.

**Safe next action:** Compare with [two-actor.preserves](../../../examples/two-actor.preserves). Check that each required record exists exactly once, has the correct schema, and is before the final step sequence. Add only the fixture that represents the scenario's intended input; an empty capability record is a meaningful deny-all case, unlike an omitted record.

**Stop condition:** If the desired authority or resource policy is unknown, stop rather than inventing it. A parser-compatible historical shape is not sufficient evidence for current execution admission.

## Symptom: unknown actor, duplicate actor, or executor rejection

**Discriminating evidence:** Registry parsing rejects duplicate IDs. Runner validation checks every step's actor references against the registry and checks executor support. Steel or Wasm without their matching reviewed executor fixture is not equivalent to a native actor declaration.

**Safe next action:** Reconcile IDs between the actual step and registry. For an admission-only scenario, preserve the existing native actor fixture rather than introducing an unrelated executor change. For an executor scenario, consult its checked-in fixture and full preflight implementation before making a claim about execution.

**Stop condition:** Do not treat adapter/remote-proxy names as proof of live integration. The executor registry accepts configured reviewed variants, but some rejection text says those kinds remain disabled. This is a scoped source-review wording discrepancy; the registry alone cannot settle whether an external effect occurred. Follow the execution path or narrow the claim to fixture evidence.

## Symptom: a send or effect is absent despite a completed report

**Discriminating evidence:** Inspect the relevant `admission-decision-v1`, its authority information and decision reason, and rollback events. Determine whether the grant is missing or policy denies an otherwise granted request. These are different boundaries.

**Safe next action:** Compare the exact actor, action, target, and value with the grant constraints. If the denial is intended, confirm absence of the prohibited runtime result rather than “repairing” it. For a clock denial, inspect that no effect request or response was recorded for that operation. For a send denial, inspect that no message was delivered.

**Worked case:** `capability_missing_send_grant_denies_delivery` supplies both actors, an explicit budget, an empty capability fixture, and one send. Its source checks valid denial evidence, validation, replay, unauthorized admission, and no delivery. Removing the capability record would instead test missing execution evidence. A review that labels both cases “send failure” loses this distinction.

**Stop condition:** A deny record alongside the prohibited effect is not a successful negative case. Preserve both observations for investigation; do not discard the effect record to make the report consistent.

## Symptom: resource divergence

**Discriminating evidence:** The runner distinguishes step-count, event-count, effect-count, and report-byte excess. Step count is checked before trace collection; event/effect checks follow each step; canonical report size is checked during construction.

**Safe next action:** Compare the intended scenario with the actual limit and usage dimension. Evidence events increase event count, and embedded input contributes to report size. Separate a deliberately over-budget test from a normal-case budget review. Any increase needs a justification tied to intended bounded work.

**Stop condition:** Do not claim every resource rejection happened before any modeled state change, and do not classify a larger limit as a remediation without checking the intended resource invariant.

## Symptom: replay or report validation rejects an existing artifact

**Discriminating evidence:** Full validation checks gate evidence, actor/executor bindings, admission, hostcalls, effect-log agreement, and usage accounting. Replay compares the embedded suite's re-execution with recorded observations. Equal final hashes do not resolve an earlier boundary mismatch.

**Safe next action:** Preserve the original report and failure artifact. Identify the first discriminating boundary and compare the embedded suite, recorded effects, and expected source revision. Use the [walkthrough](reading-and-running-a-checked-in-suite.md) for the validated command surface. Do not manually “repair” hashes or replace recorded effects with current ambient values.

**Stop condition:** An unavailable toolchain is an unavailable execution, not a passing suite. Uncertain external effects require investigation, not unconditional retry. Report actual observations separately from source-based expectations.

## Sources

- [Handbook](../README.md)
- [Distributed testing evidence and retry limits](../../distributed-testing.md)
- [Capability admission distinctions](../../technical/capabilities/capability-context-admission.md)
- [Runner preparation](../../../src/harness/parts/runner/p000/body.rs)
- [Runner budget and rollback behavior](../../../src/harness/parts/runner/p001/body.rs)
- [Executor classification](../../../src/harness/executor.rs)
- [Missing-send regression case](../../../src/harness/parts/mod/tests/m000/p001/body.rs)
- [Validation and replay](../../../src/harness/replay.rs)
