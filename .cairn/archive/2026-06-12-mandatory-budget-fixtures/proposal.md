## Why

After capability and actor registry fixtures became mandatory, the remaining implicit input for evidence-bearing harness suites is the resource budget. Old suite shapes can still execute with a default budget even though the suite did not declare its resource policy. That weakens pass evidence: a report can pass because of runner defaults rather than suite-authored resource limits.

Molten's deterministic gates should fail closed on missing resource policy just as they do on missing authority and actor/executor identity. Evidence-bearing suites must declare the budget they intend to run under, even if it is the standard local default.

## What Changes

- Require an explicit `<budget-v1 "molten.harness.budget.v1" ...>` fixture for evidence-bearing harness execution.
- Reject omitted budget fixtures before runtime turns, admission decisions, effect requests, or report generation can execute.
- Make report validation reject embedded suites that lack explicit budget evidence, even if the report contains default budget evidence produced by an older runner.
- Keep parsing compatibility for old suite shapes only as non-executable structure inspection and migration diagnostics.
- Add pass-evidence gate checks for `explicit-budget-fixture` and `no-default-resource-policy`.
- Update positive examples and tests to declare budgets explicitly.

## Impact

Resource policy becomes explicit evidence rather than a runner default. This closes the last major implicit local-suite input before deeper Nickel/Basalt policy preflight and non-native executor boundary work.
