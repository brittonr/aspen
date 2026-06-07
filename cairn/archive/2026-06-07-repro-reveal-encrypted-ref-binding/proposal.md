## Why

Encrypted-private repro bundles currently require reveal receipts before unpacking private material, but authorization can be inferred from generic secret or commitment refs. A reveal receipt reused from a nearby secret path should not authorize a repro bundle unless it explicitly names the exact encrypted-ref placeholder contained in that bundle.

## What Changes

- Add an explicit encrypted-ref binding to canonical reveal receipts used for repro unpack.
- Require `molten test repro unpack --reveal-receipt` to match every encrypted-private bundle encrypted ref by that dedicated field.
- Reject reveal receipts whose bound encrypted ref is missing, stale, unrelated to the bundle, denied, or only matches through generic secret/commitment refs.
- Preserve diagnostic/private evidence boundaries: reveal receipts authorize unpack materialization only and do not make encrypted-private bundles gate-preserving pass evidence.

## Impact

Operators get safer private repro sharing with a fail-closed binding between encrypted placeholders and reveal receipts. Existing generic reveal receipts remain parseable for non-repro consumers, while encrypted-private repro unpack requires the stronger binding.
