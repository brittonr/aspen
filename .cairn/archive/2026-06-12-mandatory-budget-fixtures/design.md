## Context

`first-class-testing-harness` added canonical budget evidence and report validation already checks that reported budget usage matches observations, effect logs, canonical report bytes, and embedded suite limits. Early compatibility behavior still allowed suites without a budget fixture to use `HarnessBudget::default()`.

After `mandatory-capability-fixtures` and `mandatory-actor-registry-fixtures`, that default budget is the remaining implicit evidence-bearing input. It affects whether a run may execute, where resource divergence occurs, and whether a report can satisfy pass gates.

## Goals

- Ensure every evidence-bearing suite has an explicit budget fixture.
- Preserve old-suite parsing where useful for diagnostics and migration, but prevent default-budget suites from executing or gating.
- Require report validation to reject embedded suites whose budget fixture was omitted.
- Require gate receipts to prove that no default resource policy was accepted.
- Keep explicit tight budgets as normal resource divergence evidence.

## Non-Goals

- Do not change budget semantics or add new resource dimensions.
- Do not remove parser support for old suite shapes.
- Do not infer budgets from profile names, actor kinds, capability grants, policy rules, or step counts.
- Do not introduce adaptive budgets in deterministic pass gates.

## Suite behavior

`parse_suite` may continue to parse old suites without a budget fixture so tools can show migration diagnostics. The parsed suite must record whether the budget was explicitly supplied.

`run_suite` and deterministic replay execution must reject suites where `budget_explicit == false` before any runtime turn, admission decision, actor side effect, ambient effect request, or report generation. This is a preflight failure, not a resource divergence, because no explicit resource policy exists.

An explicit standard local budget remains valid:

```preserves
<budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
```

An explicit tight budget also remains valid. If a run exceeds it, the runner reports normal deterministic `resource` divergence with step/expected/actual/detail diagnostics.

## Validation behavior

Report validation must reject reports whose embedded suite lacks an explicit budget fixture, even if the report includes a `<budget-v1 ...>` record with default limits. The validator must fail before treating resource usage as pass evidence because a default budget is not suite-authored policy.

A valid report must include:

- an embedded explicit `<budget-v1 ...>` fixture,
- report budget limits matching the embedded suite budget,
- usage counts matching observations, effect log entries, event counts, and canonical report bytes,
- usage that does not exceed explicit limits.

## Gate receipts

Successful pass-evidence gate receipts must include:

- `explicit-budget-fixture`
- `no-default-resource-policy`
- the existing `budget` check

These checks prove that the report used an authored resource policy rather than runner defaults.

## Migration path

Existing old-shape suites should be migrated by adding explicit budget fixtures. Local examples may use the current standard budget, but it must be present in the suite text. Negative resource tests should use explicit tight budgets; omitted budget fixtures should be reserved for tests that specifically target default-resource-policy rejection.

Future Nickel/Basalt policy integration should preserve the same invariant: resource policy may move from local Preserves fixtures to Nickel/Basalt policy refs, but missing resource evidence must fail closed.
