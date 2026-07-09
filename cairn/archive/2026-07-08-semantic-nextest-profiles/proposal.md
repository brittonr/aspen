## Why

The repository already uses nextest profiles for CI, deterministic, and exploratory runs. Those profiles mainly encode retry and timeout behavior. Reviewers still need to know which command proves pure core behavior, harness behavior, CLI behavior, distributed simulation, VM integration, or dogfood readiness.

Semantic profiles make the test harness cheaper to run locally and clearer in release review. They also prevent exploratory retries or platform diagnostics from being mistaken for deterministic pass evidence.

## What Changes

- Define semantic nextest or Nix check profiles for fast core, harness, CLI, distributed simulation, VM/platform, and dogfood/soak scopes.
- Bind each profile to evidence scope, allowed retries, expected artifacts, and release-review caveats.
- Preserve JUnit as a rendered view while canonical receipts remain normative.
- Document the smallest useful command for each risk class.

## Impact

Developers get a clear escalation path from fast pure checks to expensive platform evidence. Release reviewers get profile-specific evidence instead of one undifferentiated test pass.
