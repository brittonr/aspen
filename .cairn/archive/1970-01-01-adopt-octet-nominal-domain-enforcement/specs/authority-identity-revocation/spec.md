## MODIFIED Requirements

### Requirement: Octet guards migrated reference scopes

r[molten.authority.nominal_references.octet] Molten MUST adopt an immutable reviewed Octet cohort providing nominal-domain enforcement and the fifteen declared reference-domain catalog rows, select exact nominal denial, and guard the existing admitted core symbols at their declared physical source paths. Human-authored inventory declarations alone MUST NOT be treated as runtime enforcement. The provider-owned embedded catalog MUST remain the runtime policy authority.

#### Scenario: Raw reference regression is rejected

r[molten.authority.nominal_references.octet.guard]
- GIVEN a declared physical core scope reintroduces a selected qualified reference field as a type-correct raw `String`
- WHEN the pinned Octet cohort checks that scope with exact nominal denial
- THEN the check MUST fail with `raw_domain_value`, not merely an unrelated compiler error.
- AND the unchanged typed source MUST pass the same selected nominal checks.

## ADDED Requirements

### Requirement: Nominal catalog identifies actual adopted source scopes

r[molten.authority.nominal_references.octet.catalog] Molten MUST review all fifteen existing domain aliases and qualified typed core fields against the provider catalog using exact physical paths, primitive representation, wrapper kind, constructor policy, and explicit compatibility scopes. Logical labels MUST NOT be represented as physical migrations. Mixed wire adapters, diagnostic/display strings, unrelated same-named types, unsupported borrowed representations, and unverified tuple-field matching MUST NOT gain enforcement claims by association.

#### Scenario: Reviewed inventory matches bounded adoption

- GIVEN `PrincipalRef` through `ReceiptRef` are implemented by marker-generic checked reference families
- WHEN the consumer inventory and provider rows are reviewed
- THEN each domain MUST identify its actual admitted core fields at `crates/molten-core/src/nominal.rs` and retain explicit wire compatibility at `src/authority/nominal.rs`.
- AND the inventory MUST NOT assert wholesale node, handler, effect, retention, cache, or replay migration merely from old logical labels.

### Requirement: Nominal alias enforcement applies to actual source

r[molten.authority.nominal_references.octet.alias] A selected raw primitive alias regression MUST fail with `primitive_domain_alias` in the actual reviewed core scope. If the provider skips macro-generated aliases, the actual declaration coverage gap MUST be resolved before alias enforcement is claimed; a standalone explicit alias fixture alone MUST NOT substitute for that resolution.

#### Scenario: Primitive alias regression is independently observed

- GIVEN the actual selected alias declaration is in a source form inspected by the pinned provider
- WHEN a type-correct regression declares that domain alias as `String` in the selected core scope
- THEN exact nominal denial MUST report `primitive_domain_alias`.
- AND a Rust type mismatch or an uninspected macro expansion MUST NOT count as the negative proof.

### Requirement: Nominal enforcement preserves compatibility and lifecycle truth

r[molten.authority.nominal_references.octet.validation] Molten MUST retain exact wire DTO strings, canonical projection and roundtrip compatibility, unchanged schema identity, and explicit authority non-claims while adopting nominal checks. Acceptance MUST require recorded actual positive and negative check outcomes from the pinned cohort, the real Nix hook, review, and all native lifecycle gates; incomplete proof MUST leave the change active and unchecked.

#### Scenario: Configuration is not acceptance evidence

- GIVEN exact denial is configured but provider pinning, runtime proof, or a required gate remains unfinished
- WHEN the rollout status is reported
- THEN documentation MUST describe cohort-dependent adoption rather than assert observed enforcement.
- AND no declaration, implementation marker, pending requirement identity, or compile-fail doctest MUST authorize sync/archive or accepted coverage.
