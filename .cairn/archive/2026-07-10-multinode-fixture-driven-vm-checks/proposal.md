## Why

Multinode scenario fixtures are typed in Nickel, but the executable VM script still hard-codes scenario shape, expected artifacts, and child evidence wiring. This creates drift risk between reviewed fixture declarations and what the VM check actually runs.

## What Changes

- Use typed multinode scenario fixtures as the source of truth for VM shard plans, expected artifact kinds, unavailable policies, variance refs, and caveats.
- Export or validate fixture-derived shard plans before NixOS VM execution accepts pass evidence.
- Deny VM pass claims when observed topology, command surface, artifact kinds, child refs, unavailable policy, or caveats diverge from the fixture.
- Document how fixture authors add or update VM scenarios.

## Impact

Cluster testing becomes more reviewable and less handwritten. Fixture validation remains a planning/evidence gate; actual authority, policy, provenance, resource, source-gate, retention, and production claims stay separately gated.
