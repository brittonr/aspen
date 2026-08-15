## Why

Molten now records and validates admission decisions, but the static policy fixture still needs an explicit trust boundary before the runtime is allowed to execute steps or issue ambient effects. A report or gate should not merely trust that a `<policy-v1 ...>` fixture was parsed. It should carry evidence that the policy snapshot was canonicalized, checked as static/declarative, admitted through the policy preflight gate, and constrained so unreviewed Steel predicates cannot bypass deterministic admission evidence.

The next production path is Nickel for declarative policy/config/schema contracts, Basalt/UCAN for capability-bearing context and gate decisions, and Steel only for reviewed dynamic predicates/trusted callables. The harness slice needs to make that boundary explicit now, even while the executable policy language remains the current Preserves deny-rule fixture.

## What Changes

- Add a policy preflight gate before local harness execution can produce side effects.
- Canonicalize the embedded static policy fixture into a policy snapshot ref.
- Emit report evidence as `<policy-gate-v1 "molten.harness.policy-gate.v1" ...>` containing the policy ref, static engine, Nickel-compatible contract marker, Basalt preflight context, disabled Steel predicate list, and passed checks.
- Make report validation fail closed when policy gate evidence is missing, malformed, stale, or not bound to the embedded suite policy.
- Reject unreviewed Steel/dynamic predicate records in local harness policy fixtures.
- Extend pass-evidence gate receipts with policy preflight, Nickel static policy, Basalt policy gate, and Steel predicate review checks plus policy/policy-gate refs.

## Impact

The current Preserves policy fixture becomes a checked static policy snapshot rather than an ambient parser side effect. This preserves deterministic replay while creating a clear migration seam: Nickel can later replace the static fixture checker, Basalt/UCAN can provide real capability-bearing context, and Steel predicates can be admitted only with reviewed callable receipts without changing the fail-closed admission validation model.
