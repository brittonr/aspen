## Why

Molten already exposes nextest profiles, hermetic Nix checks, VM evidence, drift comparison, and requirement traceability. The next improvement is to make distributed testing easier to run and harder to misinterpret by separating checks by risk/cost, making traceability a required release gate, binding reproducibility metadata for every distributed shard, and keeping retry-based success out of pass evidence.

## What Changes

- Define an explicit distributed test risk matrix: fast, protocol simulation, CLI, VM smoke, VM fault, and soak/pilot.
- Expose matrix entries through documented nextest profiles, Nix checks, apps, or release-readiness commands.
- Require traceability coverage for evidence-bearing distributed requirements in CI/release review.
- Bind source, Nix inputs, test binary, seed, topology, fault plan, shard/profile, emitted receipt refs, and variance declarations to distributed test evidence.
- Enforce zero retries for release/CI pass evidence while keeping exploratory reruns and quarantine review separate.

## Impact

This change improves operator ergonomics and release review confidence without replacing subsystem gates. It should make fast failures cheap, expensive VM/soak evidence intentional, and traceability gaps visible before promotion.
