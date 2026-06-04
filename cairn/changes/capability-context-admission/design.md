## Context

The current harness has three important security/evidence properties:

1. Policy preflight evidence is required before side effects.
2. Every step records exactly one admission decision event.
3. Report validation recomputes each decision from embedded static policy evidence.

However, the admission model is still policy-only. It can represent rules such as "producer cannot assert readiness," but it does not answer the more fundamental question: did `producer` hold authority to assert anything, send to `consumer`, observe a pattern, retract an assertion, read logical time, or request randomness?

Molten's production direction is Basalt/UCAN capability-bearing enforcement. This slice introduces a deterministic local capability context fixture and validation model that can later be replaced by real Basalt/UCAN proof refs.

## Goals

- Represent capability context as canonical Preserves evidence in harness suites and reports.
- Bind admission requests and admission decisions to the capability context used for authorization.
- Make authorization deny by default when no matching grant exists.
- Validate capability evidence independently during report validation and pass-evidence gates.
- Preserve deterministic replay: same suite, capability context, policy, seed/effect-log, and runtime inputs produce the same report ref.
- Keep the first fixture simple enough for deterministic local tests while matching the future Basalt/UCAN shape.

## Non-Goals

- Do not implement full UCAN parsing, delegation chains, signature validation, revocation ledgers, or caveat evaluation in this slice.
- Do not implement real Basalt policy evaluation beyond a local deterministic capability grant checker.
- Do not add remote identity, peer key, or actor session lifecycle semantics here.
- Do not treat capability denial as an ambient process failure; expected denials can still be valid pass evidence when recorded and validated.
- Do not allow Steel predicates to mint or override capability grants without reviewed callable receipts.

## Capability fixture model

The first deterministic fixture is Preserves-shaped:

```preserves
<capabilities-v1 "molten.harness.capabilities.v1" [
  <grant "producer" "send" "consumer" #f>
  <grant "producer" "assert" #f "service.ready">
  <grant "producer" "clock" #f #f>
  <grant "producer" "random" #f #f>
  <grant "consumer" "observe" #f "service.ready">
]>
```

Grant fields are:

1. actor/principal id
2. action: `send`, `observe`, `assert`, `retract`, `clock`, `random`, later adapter/effect ids
3. target, or `#f` wildcard/absent target
4. value pattern or exact value, or `#f` wildcard/absent value

For the initial fixture, `#f` is a wildcard in grant records only. Admission request records must continue to encode absent optionals explicitly so a real boolean `#f` value is not confused with lack of evidence.

## Authorization semantics

For each suite step, the validator derives the same admission request already used by admission evidence validation. It then authorizes the request against the capability context:

- `send`: requires a grant for sender, `send`, target recipient, and body value or wildcard.
- `observe`: requires a grant for observer, `observe`, no target, and observed pattern or wildcard.
- `assert`: requires a grant for owner, `assert`, no target, and assertion value or wildcard.
- `retract`: requires a grant for owner, `retract`, no target, and retracted value or wildcard.
- `clock`: requires a grant for requester, `clock`, no target, no value.
- `random`: requires a grant for requester, `random`, no target, and bound metadata if the fixture chooses to model bounds; otherwise `upper` remains request metadata and the grant matches action authority.

No matching grant means denied authorization. Static policy deny rules are then applied as an additional policy layer. The final admission decision is deny if either capability authorization denies or static policy denies. Reasons must identify whether denial came from missing capability, static policy, or both where useful.

## Report and gate evidence

Reports must include normalized capability context evidence or a ref to it. The capability context ref must be part of deterministic run identity and must be bound to each admission decision. Gate receipts that accept reports as pass evidence must include checks such as:

- `capability-context`
- `capability-grants`
- `deny-without-capability`
- `authority-ref-binding`

Receipts should include artifact refs for the capability context and any future Basalt/UCAN proof/ref bundle.

## Validation rules

Report validation must fail closed if:

1. Capability context evidence is missing, malformed, unsupported, or not bound to the embedded suite.
2. A recorded admission decision claims allow when no matching grant exists.
3. A recorded denial reason claims missing authority but the embedded grants authorize the request.
4. A report's capability context ref is stale after the embedded fixture changes.
5. A denied effect step contains effect request/response records despite missing capability.
6. A report omits capability checks from the pass-evidence receipt.

Replay divergence at the authority boundary should be classified as `capability-decision` before downstream policy, trace, effect, or state drift when possible. If capability denial is encoded inside the admission decision event, `policy-decision` may remain a compatibility umbrella, but diagnostics should identify authority mismatch detail.

## Future Basalt/UCAN seam

The local grant fixture is intentionally not the production authority language. It is a deterministic harness fixture with the same evidence shape expected from future Basalt/UCAN integration:

- canonical context refs,
- request-bound authority proofs,
- caveat/revocation evidence refs,
- explicit denial reasons,
- fail-closed validation,
- receipt links for review and audit.

When real UCAN proofs are added, the validator should replace the local grant matcher with proof/ref validation without making missing capability evidence implicit success.
