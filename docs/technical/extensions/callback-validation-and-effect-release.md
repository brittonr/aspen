# Callback Validation and Effect Release

A successful executor return is an untrusted outcome to validate, not permission to route effects. This article follows the boundary from callback admission to adapter routing, assuming the manifest and callback vocabulary in the [system-extension runtime](../../system-extension-runtime.md). The [Technical companion](../README.md) links the surrounding lifecycle and native-recovery discussions.

## Admission precedes invocation

The deterministic core receives a `CallbackEvent`, not an ambient execution context. Its fields include callback kind, generation, event and optional payload references, accounted bytes, logical tick, deadline, and cancellation. `plan_callback_dispatch` rejects undeclared or phase-inappropriate callbacks, stale generations, malformed references, missing or expired deadlines, deadlines beyond the manifest envelope, and cancellation before producing an invocation. Resource admission can instead return a non-scheduling decision with no invocation. These mechanics are explicit in [dispatch](../../../crates/molten-core/src/system_extension/dispatch.rs).

Logical deadline validation is not an operating-system timeout. For example, the inspected comparison treats a deadline strictly less than the supplied logical tick as expired. Whether process execution stops within a physical duration belongs to the execution adapter and profile. Conflating these clocks would overstate what the pure dispatch law establishes.

When scheduling succeeds, the shell advances the invocation sequence, records an observed invocation, and calls the executor. The callback context names canonical references and logical coordinates; it does not hand out backend handles. Native code nevertheless has the trust risks of its execution profile. A narrow callback API is not evidence that trusted in-process native Rust has been sandboxed.

## Validate the whole outcome before approval

`CallbackOutcome` contains output references, typed effects, optional semantic state and checkpoint references, and health. The core validates output count, duplicate references, reference shape, and callback-specific obligations: checkpoint callbacks need a checkpoint reference, while recovery callbacks need a state reference. It then validates every typed effect. An effect names its target, operation, input and output schemas, request reference, generation, and accounted bytes.

The effect target must be an admitted fabric port, not ambient filesystem, network, clock, randomness, process, or environment access. Its `(port-id, version)` binding must exist, and its operation and schemas must belong to the manifest's requirements. Duplicate request references in the returned effect collection deny; that check is collection validation, not an across-time exactly-once mechanism.

In the [host dispatch shell](../../../src/system_extension/parts/host/p001/body.rs), invalid outcomes take the policy-violation failure path. Effect-reservation failure takes the resource-violation path. Only after successful outcome validation and reservation does the host call `commit_admitted_outcome`, release callback resources, update the semantic state reference and health, and produce a successful canonical callback receipt. Executor or commit failures instead take the executor-failure path. This ordering prevents an invalid outcome from becoming a successful approved-effect result.

## Approval and routing are different events

`route_approved_effects` requires a successful host-owned callback receipt, revalidates its approved effects against the currently admitted manifest and active generation, reserves effect-request capacity, and resolves each canonical binding before calling `FabricEffectPort::route`. It records completion evidence for returned outputs and releases the routing reservation. See the [routing implementation](../../../src/system_extension/parts/host/p002/body.rs).

This is not an atomic transaction over the entire effect list. Routing iterates through effects; an earlier provider interaction may have occurred before a later interaction fails. A returned routing error therefore cannot be read as “no external effect happened.” Conversely, successful callback evidence precedes routing and cannot prove that any provider accepted the effect.

For the materializing native profile, there is another boundary: returned bytes and effect metadata are validated and published through the value port before routing. The [native host](../../native-system-extension-host.md) distinguishes definite publication rejection from uncertainty. A content reference alone is not a substitute for those required bytes.

## Worked malicious-outcome scenario

Consider an illustrative request callback that returns a valid response reference and two effects. The first requests an admitted storage operation. The second requests ambient network access. Although the first effect is independently well-shaped, outcome validation reports the forbidden target before the host produces a successful approved-effects result. The response reference does not rescue the outcome, and the callback's own claim of success cannot authorize the network request.

Change the example so both effects validate. If storage routing succeeds and the second provider later returns an error, the callback remains evidence of an admitted outcome, while routing evidence and provider observations determine what is known about each effect. Replaying both effects indiscriminately would infer retry safety from an error that does not establish non-execution. Native intent tracking addresses this uncertainty explicitly; generic validation alone does not.

## Review and suggested verification

Review four separate observations: whether invocation occurred, whether its outcome was admitted, whether an effect was routed, and whether a provider result was observed. Examine [core tests](../../../crates/molten-core/src/system_extension/tests.rs) for cancellation, deadlines, valid typed effects, and ambient/unbound rejection. Suggested runtime review should inspect invocation counts on pre-invocation denial and confirm that invalid outcomes produce no approved routing result. Those activities are guidance, not executed evidence for this documentation change.

The [plugin lifecycle FSM](../../plugin-lifecycle-fsm.md) similarly gates shell work on an admitted decision, but its plugin hostcall machinery is not an alternative authority path for system effects.

## Limits and non-claims

Canonical receipts bind observations and admitted values; they neither grant new authority nor prove workload semantics. Output reference validation is not verification of output meaning. Effect-list validation does not provide cross-provider atomicity, durable publication, distributed delivery, or exactly-once behavior. Deployment adapters and their admitted evidence remain responsible for the external boundaries described by their profiles.

## Sources

- [System-extension runtime](../../system-extension-runtime.md)
- [Native system-extension host](../../native-system-extension-host.md)
- [Plugin lifecycle FSM](../../plugin-lifecycle-fsm.md)
- [Core callback and effect validators](../../../crates/molten-core/src/system_extension/dispatch.rs)
- [Host outcome admission ordering](../../../src/system_extension/parts/host/p001/body.rs)
- [Approved-effect routing](../../../src/system_extension/parts/host/p002/body.rs)
- [Core callback regression cases](../../../crates/molten-core/src/system_extension/tests.rs)
