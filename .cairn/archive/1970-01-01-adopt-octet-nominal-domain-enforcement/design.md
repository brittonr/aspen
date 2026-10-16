## Context

The old Molten Octet pin `fc38f59330b626961d166febfdf1a5aa6575460f` cannot enforce nominal domains. The current provider already owns the lint implementation, its schema, and generated runtime catalog. This change adopts that implementation; it does not invent a consumer-side runtime catalog loader.

## Decisions

### One runtime catalog, explicit review inventory

The provider embeds `standards/nominal-domains/generated/catalog.json`, generated from its Nickel source. Consumer `config/nominal-reference-domains.ncl` records the reviewed fifteen rows using the same fields: `domain_id`, `symbols`, `raw_representation`, `core_scopes`, `compatibility_scopes`, `compatibility_required`, `wrapper_kind`, and `constructor_policy`. Its family and alias inventory are review metadata, not alternate lint inputs. Runtime behavior must be established with the actual provider cohort.

All fifteen rows use `raw_representation = "string"`, `wrapper_kind = "marker-generic"`, and `constructor_policy = "private-fallible"`. The seven entity aliases wrap `EntityRef<Domain>`; the eight canonical aliases wrap `CanonicalRef<Domain>`. Storage is private and constructors perform syntax validation, not authority admission.

### Declare only existing physical cores

The exact core path is `crates/molten-core/src/nominal.rs`. Qualified symbols are aliases and fields in `AuthorityReferenceSet`, `ExecutionReferenceSet`, `ArtifactReferenceSet`, and `HistoricalReferenceSet`. Exact field lists belong to the checked-in inventory and governing documentation. Bare common names such as `node`, `policy`, or `value` must not replace those qualified symbols.

`src/authority/nominal.rs` is an explicit compatibility path: it contains raw wire DTOs, checked admission, projection, and additional admitted context/request structs. Compatibility takes precedence in the current provider, so this change does not claim field-level Octet protection inside that adapter. The real currentness consumer in `src/authority/parts/mod/p001/body.rs` uses admitted principals and keys, but is not itself a catalogued raw-domain scope. Other logical labels in the old profile are not physical migrated scopes.

### Respect measured checker limits

The provider's raw-value lint checks declared primitive representations and supported containers, not borrowed `&str`/`&String` APIs. `require_session` has a typed borrowed parameter/return; Rust enforces its type separation, not this catalog's String check. Heterogeneous `AdmittedReference` enum payloads remain typed, but tuple-field matching is not claimed without compiler proof. Generic wrapper storage and diagnostic text remain outside declared domain symbols.

`newtype_invariant_bypass` is currently refined-newtype-only and is not claimed as protection for these marker-generic rows. Their private storage remains a Rust boundary. The provider must minimally admit the actual `marker-generic`/`private-fallible` constructor pair in both producer validators; retaining `marker-only` and all unrelated wrapper restrictions avoids a false catalog description without inventing broader bypass checking.

The provider previously skipped macro-expanded alias declarations. Molten's existing `domains!` macro generates all fifteen aliases. The parent selected a provider-owned fix: inspect local macro aliases with resolved Rust types and use the macro source callsite for exact physical policy scope and diagnostics; keep external macro exclusions. Preserve Molten's macro and alias APIs unchanged. Actual macro-generated alias regression proof is required; changing only an explicit-source fixture does not close that mismatch.

### Cohort-dependent enforcement, not declaration-dependent readiness

Enable exact denial in `dylint.toml`. The parent pins the reviewed provider and integrates a Nix hook; no runtime success is inferred from config parsing. The hook must use the pinned Octet library/driver/toolchain, check the actual core file at its declared physical path, deny nominal checks without unrelated lint noise, and assert a type-correct String regression fails with `raw_domain_value`. A primitive alias regression must separately fail with `primitive_domain_alias`. A nonzero compiler result alone is insufficient.

## Verification design

The positive smoke compiles the actual `molten-core` library using the immutable pinned Octet cohort and exact nominal denial. A standalone isolated fixture may include the actual core source at `crates/molten-core/src/nominal.rs` to avoid unrelated lint debt, but must retain that exact catalog path and be clearly identified as the bounded positive surface, not a full-root pass.

For the negative, copy the actual core source into an isolated fixture retaining the same relative path. Change the existing `AuthorityReferenceSet::holder` field from `PrincipalRef` to `String`; do not add a shadow struct. Compile the actual module as a library with `cfg(test)` off. Its holder constructors occur only in the test module, so no core constructor or projection changes are needed for this bounded mutation. Verify plain rustc accepts it, then require the real Octet check to reject it with `raw_domain_value`. Independently prove local-macro alias enforcement using a compiler-valid `PrincipalRef = String` regression through the actual `domains!` source convention: repair only the resulting principal admission construction in the negative copy, not unrelated source APIs. Require `primitive_domain_alias`, not a missing-constructor or type-mismatch error. Keep negative material outside normal passing targets. Do not alter canonical fixtures or historical evidence.

Parent-owned commands, hook details, observed diagnostics, and canonical roundtrip outcomes must be recorded before checking validation tasks. A Rust type-mismatch failure or compile-fail doctest is not a substitute.

### Immutable cohort and remaining combined-check boundary

The implementation pins published Octet `3f7abb641ee604f54629da415bc7686f89662065` and regenerates `flake.lock` with Nix. The independent `verified-node-replication-octet` input retains the frozen pilot's original `fc38f59330b626961d166febfdf1a5aa6575460f` provider and identical complete dependency closure; all other existing root input closures are unchanged. The pinned nominal, inherited-traceability, and frozen-pilot Nix checks pass without a development override.

The recorded whole-library checks are not interchangeable: `molten` is clean, while `molten-core` reports `warning-only` with 985 findings. The user explicitly approved the focused actual-source nominal harness and clean `molten` library as this adoption's Octet acceptance scope; `evidence/2026-10-16-acceptance-scope-decision.json` retains that decision. Whole-core warning debt remains recorded and must not be described as clean or waived for other changes. All other acceptance, compatibility, review, traceability, and native lifecycle gates remain required.

## Risks and non-claims

Compatibility scope breadth intentionally bounds protection in the mixed adapter. This rollout does not migrate unrelated product APIs, implement borrowed representation checking, assert authority/freshness/evidence truth, or prove broad retention/cache/node/effect adoption. It does not authorize sync/archive while proof or any required gate is missing. No publication is part of the change.
