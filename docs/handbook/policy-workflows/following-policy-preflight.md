# Following policy preflight through a denied clock request

Mode: Walkthrough

This source-only walkthrough follows the checked-in `report_validation_rejects_effect_response_after_denial` fixture in the [harness tests](../../../src/harness/parts/mod/tests/m000/p003/body.rs). Its useful result is not a permitted clock read: it demonstrates how acceptable preflight material can describe a request that must subsequently be denied. Read the [policy preflight companion](../../technical/capabilities/policy-preflight-composition.md) for the evidence model; use the stages below to locate an actual boundary during review.

Everything here is source-checked, not executed for this guide. There are no runnable command blocks, claimed terminal results, or generated artifact hashes. The fixture is a local native-actor harness case, not an external UCAN delegation or live clock-adapter deployment.

## 1. Identify the input, without broadening it

The fixture is an inline Preserves suite named `deny-effect-validation`, with seed `1`. It supplies an explicit budget whose positional limits are `64 16 256 65536`, an actor registry containing native actor `producer`, and an explicit grant for that actor's `clock` action. Its policy contains a matching deny rule with reason `producer cannot read clock`. Its only step is a clock request by `producer`.

Keep both the grant and denial in view. Removing the grant would test a different path: missing capability authority rather than policy refusal after capability matching. The `#f` target and value entries in this fixture correspond to unconstrained optional matching fields, not fabricated resource references.

**Observable boundary:** the parsed suite retains separately identified policy, capabilities, budget, actor registry, and step. A parser failure is not a request denial and should not be reported as one.

## 2. Follow preparation before any step

In [runner preparation](../../../src/harness/parts/runner/p000/body.rs), `run_suite_inner` calls `prepare_suite_run` before `collect_trace`. Preparation requires explicit actors, capabilities, and budget; validates executor preflight inputs; and checks the number of steps against the budget. It then constructs policy, capability, and budget gate values and their snapshot references.

For this example, the explicit policy denial is valid material to describe. It does not make the policy contract itself malformed. A reviewer should therefore distinguish a gate's acceptance of material from authorization of the forthcoming clock request.

**Observable boundary:** failure here prevents trace collection. Success supplies `SuiteRunMaterial`; it does not supply blanket effect permission.

## 3. Follow the policy's normalization bindings

[Policy material construction](../../../src/harness/parts/schema/p019/body.rs) canonicalizes the policy snapshot, generates Nickel source, evaluates its JSON export, and hashes source and export as Preserves strings. The contract envelope binds backend, contract identity/version, normalized source reference, input schema, output schema, and receipt schema. Basalt envelope rejection prevents successful construction.

The corresponding source-evidence parser checks the recorded source hash, export hash, and equality between a freshly evaluated export and the recorded JSON. These are different edges: correct source addressing alone does not establish that an export came from that source.

The [Nickel cohort contract](../../nickel-toolchain.md) remains the governing toolchain reference. Nickel normalization is not policy authority, and this walkthrough does not replace that reviewed cohort with an arbitrary installed interpreter.

## 4. Keep capability preflight independent

The [capability gate implementation](../../../src/harness/parts/schema/p020/body.rs) hashes the capability snapshot and grants, builds its authority contract and preflight evidence, and emits an empty local UCAN proofset. Parsing binds capability, envelope, proofset, and grant references; validation against the suite recomputes the expected gate.

The local gate explicitly requires `fixture-authority-evidence-only`. Do not relabel the fixture grant as a verified external token. A matching canonical reference proves identity of the encoded value, not possession of live authority outside this fixture.

**Observable boundary:** capability evidence describes the supplied grant and its local provenance. Policy still has an independent opportunity to deny the request.

## 5. Locate the request denial and suppressed effect

The runner derives an admission request from the clock step and invokes `decide_with_capabilities`. In [runtime admission](../../../src/runtime/admission/mod.rs), grant authorization runs first; then policy deny rules are matched. This fixture reaches the explicit policy refusal because its grant matches.

In [runtime step dispatch](../../../src/harness/parts/runner/p001/body.rs), the denied branch rolls back the turn and returns before the allowed branch applies the step or processes replay effects. This is the critical operational boundary: merely attaching a denial annotation would be insufficient.

A completed harness run can still have report status `pass`: that means the harness completed its scenario, not that the clock action was permitted. Inspect the per-step decision and rollback evidence instead of interpreting the top-level status as authority.

## 6. Follow the negative output check

The test constructs the report, then deliberately inserts an effect request and response after the denied turn. Validation is expected to reject the altered report with the denied-effect condition. This is checked-in test intent, not a run observed while writing.

Use the example as a review procedure: retain the original suite and report identities, identify the denied step, and establish that no forbidden effect evidence can be accepted afterward. Stop at this local boundary. The fixture does not establish external clock behavior, distributed revocation delivery, or current permission for a later invocation.

## Sources

- [Handbook](../README.md)
- [Policy preflight companion](../../technical/capabilities/policy-preflight-composition.md)
- [Nickel toolchain contract](../../nickel-toolchain.md)
- [Exact denied-effect fixture](../../../src/harness/parts/mod/tests/m000/p003/body.rs)
- [Preparation and request admission](../../../src/harness/parts/runner/p000/body.rs)
- [Denied-turn dispatch and report assembly](../../../src/harness/parts/runner/p001/body.rs)
- [Policy normalization](../../../src/harness/parts/schema/p019/body.rs)
- [Capability evidence binding](../../../src/harness/parts/schema/p020/body.rs)
