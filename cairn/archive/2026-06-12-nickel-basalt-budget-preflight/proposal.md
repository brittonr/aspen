# Change: nickel-basalt-budget-preflight

## Why

Budget fixtures are mandatory, but resource policy evidence still lived in local `<budget-v1 ...>` limits plus usage checks. After policy and capability gates gained executable Nickel/Basalt preflight receipts, budget/resource policy should use the same fail-closed evidence rail.

## What

- Add `<budget-gate-v1 ...>` report evidence before runtime observations.
- Normalize explicit budget limits through deterministic Nickel resource-policy source/export evidence.
- Bind budget gates to Basalt resource contract envelopes and `<basalt-resource-preflight ...>` receipts.
- Validate budget refs, Nickel exports, Basalt receipts, and usage bindings from embedded suites.
- Add pass-evidence receipt checks/refs for resource policy preflight and budget usage binding.

## Impact

Reports gain budget-gate evidence. Older reports without budget gates no longer satisfy evidence-bearing validation, even when they include explicit budget fixtures and valid usage records.
