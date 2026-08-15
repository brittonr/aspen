## Context

`capability-context-admission` added local static grants and capability gate evidence. It intentionally allowed omitted capability fixtures to normalize to a compatibility allow-all context so existing suites could continue to run while the authority evidence rail was introduced.

That compatibility mode is not suitable for evidence-bearing runs. Molten's policy and capability model should fail closed when authority evidence is absent. The next hardening slice makes explicit capability fixtures mandatory for execution, validation, and pass-evidence gates.

## Goals

- Ensure every evidence-bearing suite has an explicit capability fixture.
- Preserve old-suite parsing where useful for diagnostics/migration, but prevent old implicit-authority suites from executing or gating.
- Require report validation to reject embedded suites whose capability fixture was omitted.
- Require gate receipts to identify that no implicit authority was accepted.
- Update example suites and tests to use least-privilege grants.

## Non-Goals

- Do not implement full UCAN proof verification.
- Do not remove the parser's ability to inspect old suite shapes.
- Do not introduce a production/legacy profile that can satisfy pass-evidence gates with implicit authority.
- Do not infer grants from actor registry entries or step contents.

## Execution behavior

`parse_suite` may continue to parse old suites without a capability fixture so tools can show diagnostics, report migration hints, or validate non-executable structure. The parsed suite must record whether capabilities were explicitly supplied.

`run_suite` and deterministic replay execution must reject suites where `capabilities_explicit == false` before any runtime turn or ambient effect request. This is a preflight failure, not a denied runtime turn, because no authority context exists from which to compute admission decisions.

An explicit empty fixture remains valid:

```preserves
<capabilities-v1 "molten.harness.capabilities.v1" []>
```

It denies all actions through normal admission evidence. This distinction is important:

- omitted fixture: invalid evidence-bearing suite;
- explicit empty fixture: valid authority context that denies every request.

## Validation behavior

Report validation must reject reports whose embedded suite lacks an explicit capability fixture, even if the report contains a capability gate record over a default allow-all context. The validator must fail before accepting admission decisions because the authority context is not part of suite identity.

A valid report must include:

- an embedded explicit `<capabilities-v1 ...>` fixture,
- a `<capability-gate-v1 ...>` record whose ref matches the fixture,
- per-step authority evidence bound to that ref,
- admission decisions recomputed from capability grants plus static policy.

## Gate receipts

Successful pass-evidence gate receipts must include:

- `explicit-capability-fixture`
- `no-implicit-authority`
- `capability-context`
- `capability-grants`
- `deny-without-capability`
- `authority-ref-binding`

Receipt artifact refs should continue to include the capability context and capability-gate refs.

## Migration path

Existing old-shape examples must be migrated by adding explicit grants. For local tests, least-privilege grants should match the steps under test. For negative/security tests, use an explicit empty fixture or intentionally missing grant rather than omitting the fixture.

Future Basalt/UCAN integration should preserve the same invariant: no evidence-bearing execution or gate may proceed without explicit authority evidence, whether that evidence is a local fixture, a UCAN proof bundle, or a Basalt receipt ref.
