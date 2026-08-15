## Phase 1: Consumer baseline and source admission

- [x] [serial] I1 Capture current Molten signature domains/preimages, payload and key refs, generation/currentness decisions, rotation behavior, issue ordering, retained authority, and positive/negative fixtures; compare them with the immutable published Molten mapping profile. r[molten.artifact_auth_adoption.source] r[molten.artifact_auth_adoption.authority]
- [x] [serial] I2 Pin Cargo and Nix to `ssh://git@github.com/OnixResearch/artifact-auth.git` revision `799459346d5416fbd7b9f55840a7371441b55afa`, generate locks only with owning tools, and reject floating, duplicate, mismatched, sibling-path, product-dependent, or license-incompatible sources. r[molten.artifact_auth_adoption.source]

## Phase 2: Adapter and cutover

- [x] [depends:molten.artifact_auth_adoption.source] I3 Add a pure adapter for domain/version, purpose, profile, payload/public-key/verifier-context refs, generation, and supplied currentness while preserving opaque-handle/backend/entropy/rotation extensions and keeping effects in Molten adapters. r[molten.artifact_auth_adoption.authority]
- [x] [depends:molten.artifact_auth_adoption.authority] I4 Dual-run legacy and standalone paths over identical positive, rotation, and tamper observations, classify every difference, reject unrelated-failure false parity, and retain legacy authority plus rollback until admission. r[molten.artifact_auth_adoption.cutover]
- [x] [depends:molten.artifact_auth_adoption.cutover] I5 Admit or reject cutover from durable compatibility evidence, update operator migration/rollback documentation, and remove duplicate canonical ownership only after the bounded rollback period. r[molten.artifact_auth_adoption.cutover]

## Phase 3: Verification

- [x] [parallel] V1 Add positive tests for valid current/overlap verification and bounded non-claims plus negative tests for wrong domain/purpose/profile/payload/context, stale generation, superseded/revoked/unknown keys, overlap signing promotion, malformed signatures, capability/membership/transport promotion, and weakened non-claims. r[molten.artifact_auth_adoption.authority] r[molten.artifact_auth_adoption.cutover]
- [x] [serial] V2 Run focused cryptographic-identity tests, exact-source checks, workspace tests, strict Clippy, Nix checks, Cairn validation/gates, accepted-spec sync, and archive with exact bounded evidence. r[molten.artifact_auth_adoption.source] r[molten.artifact_auth_adoption.cutover]
