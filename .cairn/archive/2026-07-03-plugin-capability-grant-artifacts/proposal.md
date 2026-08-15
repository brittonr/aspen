## Why

Plugin hostcall admission currently accepts generic authority/resource ref lists plus descriptor-specific matching. That preserves deny-by-default behavior, but it still leaves the authority boundary too implicit: any `blake3:...`-shaped value can be passed in an authority slot until deeper validation interprets it.

Molten should make plugin authority a first-class canonical capability grant artifact. A BLAKE3 ref can then identify the exact grant, while the grant body proves the subject, manifest, operation, descriptor, schemas, resources, policies, effects, issuer, attenuation, and revocation state that make a hostcall admissible.

## What Changes

- Define canonical `plugin-capability-grant-v1` artifacts addressed by BLAKE3 and typed as capability grant refs, distinct from artifact, schema, policy, resource, effect, and receipt refs.
- Require plugin hostcall admission to evaluate bound capability grant artifacts, not only non-empty `authority_refs`.
- Bind grants to plugin id/ref, active manifest ref, extension contract ref when applicable, hostcall descriptor ref, operation, schema refs, resource refs, effect requirements, issuer/proof refs, and policy refs.
- Add deterministic attenuation and revocation checks for scope narrowing, delegation depth, budgets, turn/tick validity, and revocation receipt refs.
- Extend Nickel authoring fixtures and Rust validation so invalid or generic refs cannot be treated as plugin authority.

## Impact

- **Specs**: `plugin-host` gains typed capability grant requirements for plugin hostcalls and lifecycle authority.
- **Core**: plugin admission becomes a pure grant-matching decision over canonical values already loaded by the shell.
- **Docs/config**: extension authors and operators get explicit typed grant contracts instead of untyped ref arrays.
- **Tests**: add positive grant-admitted hostcalls and negative fixtures for wrong ref type, wrong operation, wrong resource, stale manifest, expired/revoked grant, and missing proof evidence.
