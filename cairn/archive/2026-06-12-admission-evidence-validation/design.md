## Context

The harness/runtime slice introduced a pure admission model and canonical admission-decision events. Each step can be represented as an admission request, evaluated against an embedded static policy fixture, and either allowed or denied. Allowed steps continue to commit runtime actions or request deterministic effects. Denied turns roll back and denied effects are suppressed.

Without independent validation, however, a stale or malicious report could still claim pass evidence while omitting admission records, changing a recorded denial to an allow, or adding committed side effects after a denied decision. Because admission gates protect Molten's side-effect boundary, report validation and pass-evidence gates must treat admission evidence as mandatory and fail closed.

## Goals

- Make admission evidence mandatory for evidence-bearing harness reports.
- Verify recorded admission decisions independently from the embedded suite and policy fixture.
- Detect missing, duplicated, malformed, stale, or tampered admission records during validation and replay.
- Prove denied actions did not commit and denied effects did not issue ambient effect requests.
- Include admission checks in gate receipts so CI/release/admission decisions identify the policy evidence they accepted.
- Keep static policy declarative and canonical, with a clear migration path to Nickel contracts and Basalt/UCAN capability context.

## Non-Goals

- Do not implement the full Nickel policy engine in this change.
- Do not add reviewed Steel predicates as ordinary report data; dynamic predicates require separate review/admission boundaries.
- Do not treat the current static deny-rule fixture as the final production policy language.
- Do not replace capability enforcement; admission evidence must later compose with Basalt/UCAN authority context.
- Do not make denied behavior a process failure by default. Expected denials can be valid pass evidence when correctly recorded and validated.

## Admission evidence model

Each evidence-bearing step observation must begin with exactly one canonical admission decision record:

```preserves
<admission-decision-v1 "molten.runtime.admission-decision.v1"
  <request actor action target value upper>
  <decision status reason>>
```

The request is derived from the suite step, not from the recorded trace:

- `send`: actor is sender, action is `send`, target is recipient, value is body, upper is absent.
- `observe`: actor is observer, action is `observe`, target is absent, value is pattern, upper is absent.
- `assert`: actor is owner, action is `assert`, target is absent, value is assertion, upper is absent.
- `retract`: actor is owner, action is `retract`, target is absent, value is retracted assertion, upper is absent.
- `clock`: actor is requester, action is `clock`, target/value/upper are absent.
- `random`: actor is requester, action is `random`, target/value are absent, upper is the bound.

Absent optional fields use a canonical explicit marker. The marker must not be confused with an actual policy value. The first implementation may use `#f` in the harness fixture as a wildcard, but validator logic must keep wildcard fields local to policy matching and must compare recorded request fields against the exact derived request representation.

## Validation rules

Report validation must perform these checks for each observation before accepting the report as pass evidence:

1. The observation index matches the suite step position.
2. The observation has at least one event and the first event is an admission decision record.
3. Exactly one admission decision record appears in the observation.
4. The recorded request equals the request derived from the suite step.
5. The recorded decision equals the decision recomputed from the embedded policy fixture or policy refs.
6. An allowed non-effect step may contain committed trace records after the admission event.
7. An allowed effect step may contain one matching effect request/response pair after the admission event.
8. A denied non-effect step may contain rollback evidence after the admission event, but no committed message/assertion/retraction/observation effects.
9. A denied effect step may contain rollback or denial evidence after the admission event, but no effect request or effect response.
10. State hashes, effect log entries, and budget usage must be consistent with the validated event sequence.

Any missing, duplicated, malformed, stale, or unsupported admission record is an invalid harness report. A mismatch discovered during replay is a `policy-decision` divergence so diagnostics stop before downstream state drift.

## Gate receipt checks

A pass-evidence gate that accepts a harness report or report repro bundle must include explicit checks for:

- `admission-policy`: embedded policy fixture/schema is supported and included in suite identity.
- `admission-decisions`: each observation has exactly one recomputed matching decision.
- `deny-rollback`: denied turns do not commit runtime actions.
- `denied-effect-suppression`: denied effects do not issue effect request/response records.

These checks are in addition to report schema, effect-log, budget, actor-registry, and deterministic-replay checks.

## Static policy boundary

The first static policy fixture is intentionally simple and canonical. It is suitable for deterministic tests and early local development, but it is not the final policy engine. The production direction remains:

- Nickel for static declarative policy/config/schema gates.
- Basalt/UCAN for capability-bearing request enforcement.
- Steel only for reviewed dynamic predicates/trusted callables, with explicit receipt evidence.
- Preserves for the canonical boundary representation of policy inputs, decisions, diagnostics, and receipts.

The report validator must be structured so replacing or augmenting the static Preserves deny-rule fixture with Nickel/Basalt/Steel evidence does not remove the fail-closed admission checks.

## Failure evidence

Validation and replay failures must be emitted as canonical `<harness-failure-v1 ...>` artifacts when requested. Diagnostics should include the step index, expected and actual admission request/decision refs or rendered records where policy allows, failure kind (`invalid-harness` or `policy-decision`), and detail records such as `missing-admission`, `duplicate-admission`, `request-mismatch`, `decision-mismatch`, `denied-commit`, or `denied-effect`.
