## Context

`first-class-testing-harness` introduced canonical actor registry evidence and report validation already checks that a report's actor registry matches the embedded suite. Early compatibility behavior still allowed suites without an actor-registry fixture to infer actors from steps and assign default local native execution.

After `mandatory-capability-fixtures`, this implicit actor registry is the largest remaining suite input that can influence evidence-bearing execution without being declared. Actor identity, actor kind, and executor boundary choices affect admission requests, effect attribution, replay eligibility, and future hostcall/adapter policy.

## Goals

- Ensure every evidence-bearing suite has an explicit actor registry fixture.
- Preserve old-suite parsing where useful for diagnostics and migration, but prevent inferred registries from executing or gating.
- Require report validation to reject embedded suites whose actor registry fixture was omitted.
- Require gate receipts to prove that no inferred actors or executor fallbacks were accepted.
- Keep executor kind selection fail-closed until each non-native boundary has reviewed manifests, policy checks, and replay evidence.

## Non-Goals

- Do not implement Steel, Wasm, adapter, or remote-proxy executors in this change.
- Do not remove the parser's ability to inspect old suite shapes.
- Do not infer actor kinds from capability grants, policy rules, or step contents.
- Do not let an explicit actor registry authorize actions; capability fixtures remain the authority input.

## Suite behavior

`parse_suite` may continue to parse old suites without an actor-registry fixture so tools can show migration diagnostics. The parsed suite must record whether actors were explicitly supplied.

`run_suite` and deterministic replay execution must reject suites where `actors_explicit == false` before any runtime turn, admission decision, actor executor setup, or ambient effect request. This is a preflight failure, not a denied runtime turn, because there is no explicit actor/executor universe from which to compute trustworthy evidence.

An explicit empty registry remains valid only for suites with no actor-referencing steps:

```preserves
<actor-registry-v1 "molten.harness.actor-registry.v1" []>
```

If any step references an actor absent from the explicit registry, normal actor preflight fails with an unknown-actor diagnostic.

## Executor boundary behavior

Each actor registry entry declares an actor id and kind:

```preserves
<actor "producer" "native">
```

The first evidence-bearing profile may continue to support only `native` actors, while reserved kinds such as `steel`, `wasm`, `adapter`, and `remote-proxy` fail closed until their executor boundaries have explicit manifests and gate receipts. The runner must not silently coerce unsupported kinds to native execution.

Future executor enablement should bind registry entries to boundary evidence:

- Steel predicate/callable review receipts for Steel actors;
- Wasm component and hostcall manifest refs for Wasm actors;
- adapter manifest, effect-log, and replay/record guarantees for adapter actors;
- remote identity, transport, and non-replayable exclusion or simulation evidence for remote proxies.

## Validation behavior

Report validation must reject reports whose embedded suite lacks an explicit actor registry fixture, even if the report contains an actor-registry record inferred by an older runner. The validator must fail before accepting admission, effect, or replay evidence because inferred actor identity is not part of suite intent.

A valid report must include:

- an embedded explicit `<actor-registry-v1 ...>` fixture,
- report actor-registry evidence that matches the embedded fixture exactly,
- no actor ids in steps, observations, effects, admission requests, or final state outside the explicit registry,
- executor kind evidence that matches the declared registry and current supported executor set.

## Gate receipts

Successful pass-evidence gate receipts must include:

- `explicit-actor-registry`
- `no-inferred-actors`
- `executor-boundary`
- the existing `actor-registry` check

Receipt artifact refs should continue to identify the admitted report and suite refs. A future actor-registry gate artifact may be added if executor manifests or registry preflight evidence become separate canonical records.

## Migration path

Existing old-shape suites should be migrated by adding explicit actor entries for every actor referenced by steps, capabilities, policies, observations, and expected state. For local native examples, this means entries such as:

```preserves
<actor-registry-v1 "molten.harness.actor-registry.v1" [
  <actor "consumer" "native">
  <actor "producer" "native">
]>
```

Negative tests should omit actors from an explicit registry when testing unknown-actor failures, rather than omitting the registry fixture itself unless the test specifically targets implicit registry rejection.
