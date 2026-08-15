## Why

Source-line coverage is not enough for Molten. The important question is whether runtime boundaries were exercised: policy denial, capability grants, hostcall rejection, effect replay, redaction, resource exhaustion, adapter boundaries, and pass-evidence gates.

The accepted spec already requires boundary coverage. This change turns that requirement into an executable gate with positive and negative coverage expectations.

## What Changes

- Add a boundary coverage summary to harness reports or traceability receipts.
- Define required boundary classes for evidence-bearing suites.
- Gate changed requirements or release evidence on positive and negative boundary coverage, or explicit exemptions.
- Render unexercised boundary diagnostics for reviewers.

## Impact

Reviewers can see what semantic risk paths were actually exercised, not just that tests ran.
