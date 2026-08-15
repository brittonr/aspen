## Why

Multinode scenario fixtures are typed in Nickel, but execution wiring can still duplicate scenario shape in Rust tests or Nix scripts. That makes reviewed fixture intent drift away from what local cluster and VM harnesses actually run.

## What Changes

- Treat checked multinode scenario fixture exports as the source of truth for cluster and VM execution plans.
- Derive command surfaces, expected artifact kinds, required receipts, variance refs, unavailable policy, and caveats from fixture metadata before pass evidence is accepted.
- Add gate coverage that denies observed runs whose topology, command surface, artifact kinds, child refs, unavailable policy, or caveats diverge from the fixture.
- Keep Nickel evaluation outside runtime logic; Rust consumes checked fixture exports or equivalent checked-in fixture metadata.

## Impact

Cluster testing becomes more reviewable and less handwritten. Fixture-derived execution remains planning/evidence metadata only and does not grant authority, policy, provenance, source-gate, resource, transport, retention, deployment, or production trust.
