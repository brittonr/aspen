# Trust/crypto/secrets readiness decision

Package-scoped promotion is complete for the three real reusable candidates:

- `aspen-crypto`: promoted to `extraction-ready-in-workspace` for default/no-default transport-free crypto helpers.
- `aspen-trust`: promoted to `extraction-ready-in-workspace` for the default/no-default pure helper/state/wire surface.
- `aspen-secrets-core`: promoted to `extraction-ready-in-workspace` for portable secrets constants and KV/Transit/PKI persisted state/config contracts.

The aggregate `trust-crypto-secrets` family remains `workspace-internal`. The decision deliberately does not promote `aspen-secrets`, `aspen-secrets-handler`, `aspen-crypto/identity`, or `aspen-trust/async`; those remain runtime/service/compatibility surfaces that need a future explicit slice before any package-level promotion.

Evidence basis:

- checker coverage validates the three real package candidates and passes with no failures/warnings;
- default/no-default dependency graphs exclude runtime transport/storage/handler shells;
- serialization-contract tests pin current stable trust and secrets-core contracts;
- compatibility checks preserve runtime consumers without making them reusable promotion candidates.
