## Why

Molten already has a runtime-managed Iroh protocol router boundary, and main's ALPN registry discipline is worth adapting. Without a canonical registry, protocol identifiers can drift across docs, router admission, handler installation, tests, and operator diagnostics. Duplicate or malformed ALPNs can also blur ownership and make unsupported-protocol denials less actionable.

Molten should define a reviewed ALPN routing registry that names each protocol namespace, owner, handler profile, admission requirements, resource limits, compatibility status, and non-authority boundary before live Iroh handlers are advertised.

## What Changes

- Define canonical ALPN registry records for Molten-owned Iroh protocols.
- Gate router install/replace/remove operations against registry ownership, uniqueness, formatting, handler-profile, and generation rules.
- Require unsupported, duplicate, stale, malformed, or wrong-owner ALPN attempts to deny before frame delivery or handler mutation.
- Add positive and negative fixtures proving ALPN routing evidence remains transport/routing evidence only.

## Impact

- **Files**: node-runtime specs, testing harness fixtures, runtime router docs, Iroh adapter shells, operator diagnostics, and future ALPN handler registration code.
- **Testing**: positive fixtures for admitted registry entries and handler replacement; negative fixtures for duplicate ALPN bytes, malformed identifiers, wrong owner namespace, stale generation, unsupported ALPN, handler-profile mismatch, and transport-as-authority overclaims.
- **Security**: ALPN selection and endpoint identity route bytes to a handler, but do not grant operation authority, policy admission, provenance, resource, source-gate, retention, or execution trust.