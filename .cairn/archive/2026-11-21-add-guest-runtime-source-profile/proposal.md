## Why

Full Aspen source admission measures a 21,247-path / 85 GB build closure. A ChaosControl guest cannot carry that closure under its bounded image and memory limits. The guest needs one declared, signed runtime-only cohort that is honest about what it does not prove.

## What Changes

- Add the distinct `molten-nix-guest-runtime-binding-v1` producer profile to the pure startup-binding admission.
- Add an operator producer that builds and signs an input-addressed guest output whose `build-closure` and all six graphs contain only the measured runtime cohort.
- Keep the full `molten-nix-source-binding-v1` profile unchanged and reject every other producer string.
- Require guest evidence to state that the profile proves runtime identity and execution, not full build provenance, compiler/gate closure completeness, or production authority.

## Impact

- **Surfaces**: a pure profile constant/admission case, a guest-binding producer, and guest-only source trust.
- **Scope**: no change to full-profile validation, trust schema, Nix verifier, or runtime authority.
- **Claims**: guest output proves exactly the runtime paths it measures and the exact executing binary; it does not prove the omitted build closure.
