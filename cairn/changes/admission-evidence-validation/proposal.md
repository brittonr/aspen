## Why

Molten now has a first in-process admission gate: every harness step can produce a canonical admission decision, denied turns roll back before commit, and denied effects do not emit ambient effect requests. That is necessary but not enough for CI, release, upgrade, or policy-admission evidence.

A report validator or gate must not trust a rendered `pass`, a trace that merely happens to contain an admission record, or the runner that originally produced the report. It must independently check that admission evidence is present, shaped correctly, bound to the embedded suite step, recomputed from the embedded policy, and consistent with committed actions and effect records. Missing or malformed admission evidence must fail closed.

## What Changes

- Define fail-closed validation for admission decisions in harness reports.
- Require each evidence-bearing step observation to contain exactly one canonical admission decision record before semantic trace/effect records.
- Bind admission decision records to the suite step: actor, action, target, value, and effect metadata must match the canonical step input.
- Recompute the decision from the embedded static policy fixture or policy refs rather than trusting the recorded decision.
- Require denied turns to contain only admission evidence plus rollback evidence; no committed messages, assertions, retractions, effect requests, or effect responses may appear after a deny.
- Require denied effects to suppress ambient effect requests and responses entirely.
- Strengthen report validation, deterministic replay, and gate receipts with explicit admission-policy and admission-decision checks.
- Emit canonical validation failures and first-divergence diagnostics for missing, stale, malformed, or tampered admission evidence.
- Keep the first static policy format Preserves-shaped while preserving the path to Nickel contracts for declarative policy and Steel only for reviewed dynamic predicates.

## Impact

Admission evidence becomes a first-class pass-evidence gate rather than an incidental trace. This makes the current Preserves policy fixture safe enough to evolve toward Nickel-backed static contracts, Basalt/UCAN capability checks, and reviewed Steel predicates without weakening the deterministic replay law or accepting side effects with missing evidence.
