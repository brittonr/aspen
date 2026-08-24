# Proposal: Adopt the Nickel 1.17 evaluator cohort

## Why

Molten embeds `nickel-lang 2.1.0`, which uses `nickel-lang-core 0.17.0`. Its Nix command-line tool currently resolves to Nickel `1.16.0`.

Policy and runtime configuration must not depend on different evaluator cohorts across embedded and command-line paths.

## What Changes

- Update the embedded library to `nickel-lang 2.2.0` and its `nickel-lang-core 0.18.0` cohort.
- Pin the command-line evaluator to Nickel `1.17.0` at exact upstream commit `1320a983e6c3d1e2fb53dd2464b084b4903b1426`.
- Re-run policy, configuration, receipt, and runtime-profile fixtures under the new cohort.
- Add negative fixtures for changed diagnostics, rejected contracts, imports, and malformed values.
- Record the evaluator cohort in bounded release evidence.

## Impact

Molten configuration and policy evaluation will use one reviewed Nickel cohort. Molten retains runtime, authority, persistence, evidence, and release decisions.

## Dependencies

This change uses the upstream Nickel `1.17.0` release cohort.

## Non-goals

- Do not change Molten runtime semantics or authority policy.
- Do not treat evaluator alignment as policy correctness.
- Do not update unrelated dependencies only to obtain Nickel.
- Do not weaken existing fail-closed behavior.
