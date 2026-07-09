## Why

Molten already has reviewed production-profile contracts, but the node daemon still synthesizes most runtime configuration from local defaults during `node init` and `node run`. Operators need a profile-backed path that turns reviewed configuration artifacts into startup evidence without relying on ad hoc CLI flag repetition or hidden Rust defaults.

Profile-backed node configuration should preserve the existing safety boundary: Nickel remains authoring-time validation, while runtime Rust consumes checked exports, canonical Preserves values, and receipts.

## What Changes

- Add a profile-backed node configuration path for `molten node init`, `run`, and `serve` that consumes checked exported profile data or profile evidence refs.
- Bind selected adapter profiles, state layout, source-gate inputs, resource limits, live transport settings, and profile metadata into node config and startup receipts.
- Keep current local fixture defaults available for development, but label them as local-fixture configuration rather than production profile evidence.
- Define explicit override rules for CLI-supplied values versus profile values, with receipt diagnostics for every override.
- Add positive and negative tests for valid profile-backed startup, missing profile evidence, unsupported adapter vocabulary, stale/tampered profile refs, and forbidden runtime Nickel evaluation.

## Impact

- **Files**: node runtime config core, node daemon init/run/serve shell, production profile parsing/admission helpers, CLI docs, startup receipts, and tests.
- **Testing**: unit tests for pure profile resolution, CLI integration tests for profile-backed init/run, and negative fixtures for stale or malformed profile evidence.
- **Safety**: profile evidence remains non-authoritative. It does not grant authority, source-gate acceptance, adapter readiness, resource sufficiency, retention clearance, transport correctness, or release eligibility by itself.
