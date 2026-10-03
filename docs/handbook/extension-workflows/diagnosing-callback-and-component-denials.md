# Diagnosing callback and component denials

Mode: Troubleshooting

Start by locating the last boundary supported by evidence: admission, invocation, process exit, outcome decoding, value publication, effect routing, or component receipt stage. “The extension failed” is too broad to justify a retry. The [callback validation companion](../../technical/extensions/callback-validation-and-effect-release.md) explains the boundaries; this page selects evidence and safe next actions. Navigation: [Handbook](../README.md).

**Evidence status:** cases below are source-backed diagnoses and checked-in test scenarios, not reproduced runtime failures in this documentation batch. Preserve original artifacts and adapter observations. Do not delete state, weaken admission, or change profiles merely to get past a denial.

## Symptom: a selected fixture profile is refused

**Discriminating evidence:** the CLI enum accepts the spelling `native-process`, while the deterministic fixture immediately returns an error for that profile. This is distinct from executable admission, spawn failure, or native callback decoding: none of those stages is reached by this fixture path.

**Safe next action:** use the deterministic fixture only for `in-process-native` or `sandboxed-component`, or inspect the separate native integration scenario for the native service composition. Record this CLI/implementation surface mismatch as a source-review observation, not a newly reproduced bug.

**Stop condition:** do not invent a native fixture deployment command. The inspected extension CLI has only fixture execution and status readback operations.

## Symptom: no callback invocation is observed

**Discriminating evidence:** inspect callback declaration, phase, generation, event/payload reference shape, logical tick, deadline, and cancellation. `plan_callback_dispatch` rejects invalid events before returning an invocation. Resource admission may instead return a non-scheduling decision with `invocation: None`.

**Safe next action:** compare the event to the exact admitted manifest and current lifecycle snapshot. Separate stale generation from overload and deadline violations; each has a different missing prerequisite. The deadline comparison is against logical coordinates, not elapsed process time.

**Stop condition:** do not increase resource limits or rewrite generation values to make an old event look current. Any newly admitted event must have its own valid context.

## Symptom: native child exits but callback admission fails

**Discriminating evidence:** `accept_execution_receipt` first requires an exited process, accepted exit policy, and nontruncated stdout. Only then does it decode the bounded native outcome and validate generic callback obligations. A process receipt can therefore exist without an accepted callback result.

The independent native fixture refuses inherited `HOME`; it expects one packed canonical envelope on stdin. It writes protocol bytes to stdout and diagnostics to stderr. Malformed output, trailing bytes, wrong identities, absent materialized values, or ambient effect targets are separate denial reasons. The wire tests explicitly cover trailing data, oversized data, ambient process effects, and reference-only or substituted values.

**Safe next action:** retain the exact bounded stdout observation and admitted framing/value limits. Check producer framing before inspecting application semantics. For environment failures, review the admitted execution adapter's clearing behavior; do not manually launch a different unadmitted process as replacement evidence.

**Stop condition:** do not strip arbitrary suffixes from a frame, accept truncated output, or reinterpret text as packed Preserves.

## Symptom: publication fails after process success

**Discriminating evidence:** distinguish `RejectedBeforeAcceptance` from `UnknownAfterAcceptance`. The latter can mean bytes were published even though a definite result was lost. The executor marks uncertain publication unknown and propagates that uncertainty to callback completion.

**Safe next action:** inspect callback and publication operations together using the [journal procedure](inspecting-a-native-callback-journal.md). Identify the missing definite observation and retain the unresolved references for reconciliation.

**Stop condition:** no unconditional callback replay or dependent provider routing. Finding bytes with matching identity is not proof that downstream effects did or did not occur.

## Symptom: provider routing happened, but no completion callback appears

**Discriminating evidence:** the native integration test supplies missing, identity-mismatched, and oversized provider outputs. In each case the provider is routed once, its effect becomes terminal, and callback observations do not grow. That source case distinguishes completion-value admission failure from an unstarted effect.

**Safe next action:** inspect provider output identity, materialized bytes, admitted size bound, completion generation, and port binding. Preserve terminal provider evidence separately from the failed delivery attempt.

**Stop condition:** do not reroute the provider just to regenerate its output. Terminal provider state does not prove workload success; the extension still owns its semantic transition.

## Symptom: a component is denied before execution

**Discriminating evidence:** examine requested profile, artifact header, WIT world/cohort, production materialization, imports, and resource facts. Component tests reject production loose bytes and core modules without fallback. Admission tests reject unsupported features, dynamic growth, undeclared WASI, and unused authority grants. The shell independently reinspects actual resource declarations.

**Safe next action:** correct the supplied cohort or bundle through its normal admission owner. A deterministic system-extension `WasmProbe` success cannot resolve a Component Model denial: the probe is a different execution path.

**Stop condition:** no ABI reinterpretation, ambient WASI, or test-only materialization presented as production evidence.

## Worked case: distinguish output denial from fuel exhaustion

The component shell tests include an invalid-output component and a fuel-exhaustion component. Both reach inspection and instantiation, then produce a denial instead of an execution receipt. Their typed classes differ: `invalid-preserves-payload` versus `fuel-exhausted`.

For invalid output, investigate the canonical payload producer. For fuel exhaustion, inspect the admitted workload and bound without silently expanding authority or claiming an engine defect. Raw diagnostic text is not the canonical classification: another checked-in test ensures guest text mentioning fuel cannot spoof the typed denial class. Record the actual stage chain and class, plus diagnostics separately. A receipt establishes a bounded observation, not correctness or permission to retry.

## Sources

- [Handbook](../README.md)
- [Callback validation and effect release](../../technical/extensions/callback-validation-and-effect-release.md)
- [Native host contract](../../native-system-extension-host.md)
- [Component runtime contract](../../wasm-component-runtime.md)
- [Pure callback dispatch](../../../crates/molten-core/src/system_extension/dispatch.rs)
- [Native result admission](../../../src/system_extension/native_host/parts/executor/p002/body.rs)
- [Independent callback producer](../../../src/bin/molten-native-extension-fixture.rs)
- [Native wire and publication tests](../../../src/system_extension/native_host/tests.rs)
- [Provider-output and process failure cases](../../../tests/parts/nativesystemextension/p001/body.rs)
- [Component shell denial cases](../../../src/wasm/component/tests/shell.rs)
- [Component admission cases](../../../src/wasm/component/tests/admission.rs)
- [Fixture profile restriction](../../../src/system_extension/parts/fixture/p000/body.rs)
