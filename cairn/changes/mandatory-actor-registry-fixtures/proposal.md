## Why

The harness now requires explicit capability evidence, but actor identity and executor selection can still be inferred from old suite shapes. Inferred actor registries are another ambient input: a report can appear evidence-bearing without proving which actors existed, which executor kind was selected for each actor, or whether non-native execution boundaries were reviewed before side effects.

Molten's actor model should fail closed at the same boundary as capabilities and policy. Evidence-bearing suites must declare their actor registry explicitly, even when the registry is intentionally empty for an empty suite.

## What Changes

- Require an explicit `<actor-registry-v1 "molten.harness.actor-registry.v1" ...>` fixture for evidence-bearing harness execution.
- Reject omitted actor registries before runtime turns, admission decisions, or ambient effect requests can execute.
- Make report validation reject embedded suites that lack explicit actor registry evidence.
- Keep parsing compatibility for old suite shapes only as non-executable structure inspection and migration diagnostics.
- Add pass-evidence gate checks for `explicit-actor-registry`, `no-inferred-actors`, and `executor-boundary`.
- Treat executor kind as an evidence boundary: unsupported or unreviewed Steel, Wasm, adapter, or remote-proxy actors cannot satisfy deterministic pass gates by falling back to native or inferred execution.

## Impact

Actor identity and executor selection become explicit evidence rather than runner defaults. This closes the next implicit-input hole before Steel actors, Wasm components, adapter-backed services, or remote proxies can participate in evidence-bearing profiles.
