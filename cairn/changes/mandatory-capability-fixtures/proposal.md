## Why

The first capability-context implementation introduced canonical `<capabilities-v1 ...>` fixtures, capability-gate evidence, authority-bound admission decisions, and deny-by-default behavior for explicit fixtures. It still preserved compatibility by normalizing omitted capability fixtures to a local allow-all context. That kept old suites passing, but it also left an authority gap: a report could be evidence-bearing without proving that authority was explicitly declared.

Molten's admission model should not infer authority from actor names, old suite shape, or runner defaults. Evidence-bearing suites must name their capability context explicitly, even if the context is intentionally empty and denies every side effect.

## What Changes

- Require an explicit `<capabilities-v1 "molten.harness.capabilities.v1" ...>` fixture for evidence-bearing harness execution.
- Reject omitted capability fixtures before runtime turns or ambient effect requests can execute.
- Make report validation reject embedded suites that lack explicit capability fixtures.
- Update examples and tests to declare least-privilege capability grants.
- Keep parsing compatibility for old suite shapes only as non-executable structure inspection; they cannot satisfy pass-evidence gates without explicit capabilities.
- Add pass-evidence gate checks for `explicit-capability-fixture` and `no-implicit-authority`.

## Impact

Capability evidence becomes mandatory rather than a compatibility default. This closes the largest remaining authority hole before real Basalt/UCAN proof validation, Wasm hostcalls, Steel actors, adapters, or remote proxies are admitted into evidence-bearing profiles.
