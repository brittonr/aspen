## Why

The project depends on `preserves-schema`, but trust-boundary validation is still mostly manual `IOValue` record parsing. Manual parsers are useful for early iteration, yet they duplicate shape rules and make it harder to prove that every external record family enforces the same canonical schema contract.

## What Changes

- Introduce schema-backed validation for selected high-risk Preserves boundary records.
- Check in versioned schema artifacts and bind their refs in receipts or diagnostics.
- Start with node-control ingress, plugin hostcalls, evidence-chain bundles, retention receipts, and release evidence bundles.
- Preserve ergonomic Rust DTOs internally while making external acceptance depend on schema validation plus existing semantic gates.

## Impact

- **Files**: schema artifacts, `preserves_rail` schema adapter, node runtime, plugin host, evidence chain/Iroh exchange, retention, operator dogfood, and tests.
- **Testing**: valid fixtures pass schema validation; malformed/missing-field/wrong-type/extra-critical-field inputs deny before semantic side effects.
