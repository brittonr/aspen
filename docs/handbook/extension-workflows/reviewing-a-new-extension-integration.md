# Reviewing a new extension integration

Mode: Review checklist

Use this checklist to review a concrete integration change or evidence packet. Each acceptance answer should name an artifact, source boundary, or observed scenario; “uses the framework” is not sufficient. The governing [system-extension runtime](../../system-extension-runtime.md) and [native host contract](../../native-system-extension-host.md) define requirements. The [callback validation companion](../../technical/extensions/callback-validation-and-effect-release.md) supplies theory. Navigation: [Handbook](../README.md).

**Review status:** this checklist is source-checked guidance, not a completed review or executed verification. Checked-in tests cited below identify relevant scenarios, not passing results for a candidate integration.

## 1. Identify the claimed deliverable

- [ ] Does the packet name the extension, service, exact implementation identity, admitted execution profile, and intended consumer? Require independent identity evidence, not just a path or service name.
- [ ] Does it distinguish a generic fixture, native local pilot, and production component-profile execution? A `sandboxed-component` deterministic fixture uses a core-module probe while Rust constructs outcomes; that is not proof of WIT component integration.
- [ ] Is the claim bounded to what was exercised? Record whether the submitted evidence includes source review, actual process execution, adapter routing, durable journal reopen, or component materialization. Do not promote one category into another.

An acceptable scope statement makes unavailable stages explicit. For example, local callback execution with a conformance value port does not establish deployment durability or distributed availability.

## 2. Check admission joins, not just individual shapes

- [ ] Are capability, policy, provenance, evidence-profile, executable, and resource references supplied by their actual owners? Repeated-letter fixture hashes are not installation evidence.
- [ ] Does system-tier admission cover the requested fabric authorities? Plugin permissions and possession of executable bytes cannot substitute for it.
- [ ] Does each required port match exact version, operation, input/output schemas, authority, resources, replay class, and implementation profile? If an optional port is incompatible, is the integration denied rather than silently redirected?
- [ ] Does the executor advertise the admitted profile? For native execution, do execution-port, host-profile, executable, and callback-context identities agree?

Retain the mismatching pair when rejecting a join. A list of individually canonical artifacts is not enough if they belong to different cohorts.

## 3. Demand pre-invocation and outcome evidence

- [ ] Is there evidence that stale generation, cancellation, invalid phase, and deadline denial prevent invocation? Use the actual invocation observation boundary, not just an error string.
- [ ] Are logical deadlines distinguished from process timeout and teardown bounds?
- [ ] Does the host validate the whole outcome before returning approved effects? Review an outcome containing both a legitimate response and a forbidden effect; the legitimate portion must not authorize the remainder.
- [ ] Are checkpoint and recover obligations enforced, and are duplicate or malformed references rejected at the appropriate boundary?
- [ ] Does an accepted callback remain distinguishable from an effect route and a provider observation? A successful callback receipt is not effect execution evidence.

For native callbacks, additionally require exact reference-and-byte values, bounded framing, nontruncated stdout, and accepted exit policy. The independent fixture producer and host decoder are useful interoperability boundaries, but fixture semantics do not prove candidate semantics.

## 4. Inspect every uncertainty window

- [ ] Is callback intent saved before materialization and execution? Is publication intent saved before each value-port publication? Identify both the code ordering and candidate adapter guarantees.
- [ ] Does the integration preserve definite rejection versus uncertain acceptance? Review a publication that may have succeeded before its observation was lost.
- [ ] Are unresolved callback, ingress, publication, and effect operations retained with generation and parent references? Does the recovery inventory keep retry permission disabled?
- [ ] If provider output admission fails after routing, does the provider effect remain terminal without a second route? Require evidence that completion delivery did not occur with missing or altered bytes.
- [ ] Does journal durability remain separate from value durability? Redb journal persistence does not certify an in-memory value port as a deployment adapter.

No collection deduplication, intent identity, or completion-consumption check establishes exactly-once external effects by itself.

## 5. Review continuity and shutdown

- [ ] Are semantic state and lifecycle checkpoint separately identified and recoverable under the admitted state schema?
- [ ] Do upgrade and rollback advance generation sequentially and use the named checkpoint and admitted compatibility plan? Old-generation messages and completions must not cross into the new generation.
- [ ] Is retryable restart bounded, and are policy/resource/fatal failures treated according to quarantine rules rather than an indefinite loop?
- [ ] Does drain stop new ingress, and does removal wait for idle resources, terminal lifecycle, and no unresolved work? Reject proposals to delete records to satisfy these conditions.

## 6. Add component-specific acceptance evidence

- [ ] Does artifact classification select Component Model rather than reinterpret a core module? Are profile, WIT world/package/source, toolchain, and feature cohort consistent?
- [ ] Are production bytes remeasured from complete Mantle materialization with external evidence roles, rather than loose test bytes?
- [ ] Are imports within the reviewed profile? The first cohort grants no imports or WASI; a plausible authority grant does not enable an unsupported interface.
- [ ] Are actual memory/table/instance facts reinspected, growth bounded as required, and canonical output validated? Require typed denial evidence for invalid output and fuel exhaustion, not classification inferred from raw engine text.

## Worked review disposition

A candidate packet contains a successful generic fixture run, a native process receipt, and a request reference without its bytes. Accept the fixture only as evidence of its exercised host path. The process receipt does not establish native outcome admission, and the missing bytes fail the materialized-value cohort. Request exact value evidence and publication observations through the admitted adapter; do not approve reference-only fallback or rerun uncertain effects.

Close the review with accepted claims, rejected claims, missing named evidence, and executed scenario results supplied by the integration owner. Keep source-review observations separate from reproduced failures. Receipts remain evidence, not authority or a release decision.

## Sources

- [Handbook](../README.md)
- [System-extension runtime](../../system-extension-runtime.md)
- [Native host contract](../../native-system-extension-host.md)
- [Callback validation and effect release](../../technical/extensions/callback-validation-and-effect-release.md)
- [Component runtime contract](../../wasm-component-runtime.md)
- [Native intent ordering and identity joins](../../../src/system_extension/native_host/parts/executor/p001/body.rs)
- [Native result admission](../../../src/system_extension/native_host/parts/executor/p002/body.rs)
- [Native lifecycle and ingress integration cases](../../../tests/parts/nativesystemextension/p000/body.rs)
- [Provider-output denial cases](../../../tests/parts/nativesystemextension/p001/body.rs)
- [Component execution and denial cases](../../../src/wasm/component/tests/shell.rs)
- [Recovery and removal laws](../../../crates/molten-core/src/system_extension/native_host/recovery.rs)
