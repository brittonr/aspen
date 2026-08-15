# Design: Nickel/Basalt budget preflight

## Context

The harness budget frontend remains the Preserves `<budget-v1 ... <limits ...>>` fixture. Mandatory budget fixtures removed default resource policy, but the fixture still needed a static preflight receipt before it could serve as pass evidence.

## Nickel resource-policy boundary

For each suite budget, the runner derives deterministic Nickel source containing:

- `schema_version = "molten.harness.budget.nickel-static.v1"`
- the harness budget schema id
- the canonical budget fixture ref
- step, effect, event, and report byte limits

The runner evaluates/exports this source with `nickel-lang` and records source/export refs in `<budget-gate-v1 ...>`. Report validation re-runs the export and rejects stale or tampered Nickel resource-policy evidence.

## Basalt resource boundary

The runner constructs a Basalt `ContractEnvelope` with backend `nickel`, contract id `molten.harness.resource-budget`, version `v1`, the Nickel source ref as normalized source hash, the budget schema as input schema, the budget usage schema as output schema, and the Basalt resource preflight receipt schema. It validates the envelope before report generation.

The budget gate embeds a `<basalt-resource-preflight ...>` receipt with decision, backend, contract id, envelope ref, budget ref, normalized source ref, and Basalt reason. Validation checks all bindings and recomputes the expected gate from the embedded suite budget.

## Usage binding

Budget evidence still records actual usage after the report stabilizes. Validation checks that usage matches observations, effect log, and canonical report bytes, and that it remains within limits. The budget gate contributes the static resource-policy preflight; the budget evidence contributes dynamic usage.

## Gate receipts

Pass-evidence receipts include artifact refs for budget, budget gate, Nickel resource source/export, and Basalt resource preflight receipt. Checks include `resource-policy-preflight`, `nickel-resource-policy`, `nickel-resource-export`, `basalt-resource-receipt`, and `budget-usage-binding`.
