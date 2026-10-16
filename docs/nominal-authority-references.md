# Nominal authority and artifact references

## Result and adoption status

Molten admits selected authority and artifact strings into distinct Rust reference domains. Preserves remains the canonical wire authority. The Rust types prevent selected local category substitutions after admission; their existence does not establish Octet enforcement.

`dylint.toml` selects `nominal_domain_mode = "deny"`. The immutable `octet-toolchain` input pins the reviewed, published provider `3f7abb641ee604f54629da415bc7686f89662065`. Its real Nix runtime hook passes: the unchanged actual core source is clean, a type-correct raw field fails with `raw_domain_value`, and a type-correct local-macro alias fails with `primitive_domain_alias`. The whole `molten` library is also clean. The separate whole-core diagnostic reports `warning-only` with 985 findings; it is not a clean producer and is not this adoption's acceptance gate under the explicit owner decision below.

The native adoption is synced into the accepted authority-reference specification and archived at [`.cairn/archive/1970-01-01-adopt-octet-nominal-domain-enforcement/`](../.cairn/archive/1970-01-01-adopt-octet-nominal-domain-enforcement/). The user explicitly [approved focused nominal acceptance](../.cairn/archive/1970-01-01-adopt-octet-nominal-domain-enforcement/evidence/2026-10-16-acceptance-scope-decision.json): the pinned actual-source nominal harness and clean `molten` library, with all other compatibility, traceability, review, and native lifecycle gates unchanged. Native execution and identity-linkage receipts are retained in `evidence/cairn-source-remediation-2026-10-15/attempt-12-focused-nominal-closeout/`. This worktree closeout does not claim canonical-checkout integration or authorize a Molten remote push.

## Reviewed fifteen-domain inventory

<!-- r[impl molten.authority.nominal_references.inventory] -->
<!-- r[impl molten.authority.nominal_references.inventory.complete] -->

All aliases and fields below physically live in `crates/molten-core/src/nominal.rs`. `A`, `X`, `B`, and `H` abbreviate `AuthorityReferenceSet`, `ExecutionReferenceSet`, `ArtifactReferenceSet`, and `HistoricalReferenceSet` respectively. The Nickel inventory spells every symbol in full; abbreviated names are not catalog selectors.

| Domain | Rust alias | Family | Actual typed core fields |
|---|---|---|---|
| principal | `PrincipalRef` | entity | `A::holder` |
| node | `NodeRef` | entity | `X::node` |
| actor | `ActorRef` | entity | `X::actor` |
| service | `ServiceRef` | entity | `X::service` |
| session | `SessionRef` | entity | `A::session`, `X::session` |
| authority-context | `AuthorityContextRef` | entity | `A::context` |
| delegation | `DelegationRef` | canonical BLAKE3 | `A::delegation` |
| revocation | `RevocationRef` | canonical BLAKE3 | `A::revocation` |
| key | `KeyRef` | canonical BLAKE3 | `A::key` |
| policy | `PolicyRef` | canonical BLAKE3 | `A::policy` |
| resource | `ResourceRef` | canonical BLAKE3 | `A::resource`, `X::resource` |
| evidence | `EvidenceRef` | canonical BLAKE3 | `A::evidence`, `B::evidence`, `H::evidence` |
| artifact | `ArtifactRef` | canonical BLAKE3 | `B::artifact`, `H::artifact` |
| operation | `OperationRef` | entity | `X::operation`, `B::operation` |
| receipt | `ReceiptRef` | canonical BLAKE3 | `B::receipt`, `H::receipt` |

The old labels `authority-holder`, `node-control`, `handler-admission`, `capability-proofset`, `effect-admission`, `artifact-binding`, `retention`, `cache`, `provenance`, and `replay` were logical role descriptions, not physical source scopes. This inventory does **not** establish wholesale migration of those product implementations. In particular, the execution/artifact/historical reference sets do not by themselves prove all live node, effect, retention, cache, or replay paths use these aliases. Unrelated `prolly_map::NodeRef` and `artifact_auth_core::ArtifactRef` are different types and are not this catalog's domains.

## One provider policy, explicit physical boundaries

`config/nominal-reference-domains.ncl` is a review inventory, not an Octet runtime policy input. It mirrors the provider's existing row schema, with IDs `molten.nominal-reference.<domain>`, exact aliases and qualified field symbols, `raw_representation = "string"` (Rust `String`), `wrapper_kind = "marker-generic"`, and `constructor_policy = "private-fallible"`. Octet executes its embedded `standards/nominal-domains/generated/catalog.json`, generated from provider-owned Nickel. Consumer and provider rows must agree before adoption is accepted.

For every row:

- Exact core scope: `crates/molten-core/src/nominal.rs`.
- Explicit compatibility scope: `src/authority/nominal.rs`.
- Compatibility required: `true`; the wire strings are deliberately retained.

Scopes are exact normalized physical paths (also matched as suffixes of absolute source paths), not directories, globs, or logical labels. Compatibility takes precedence. The mixed wire adapter is therefore not claimed as a field-level Octet-protected core scope.

The generic families `EntityRef<D>` and `CanonicalRef<D>` own private text storage, marker domains, checked constructors, `as_str`, `domain`, and consuming conversion. Entity construction uses a bounded lowercase grammar; canonical construction requires lowercase `blake3:` references with the exact digest length. Construction checks category syntax only.

Validation helpers, supplementary APIs, and existing unit tests are textually included from `crates/molten-core/src/parts/nominal/p000/body.rs`. The registered aliases and admitted-set field declarations remain in the exact catalogued core file.

The provider originally skipped macro-expanded aliases. The local provider repair inspects local `domains!` aliases using resolved Rust types and the physical source callsite, while retaining external macro exclusions. The actual-source negative now produces `primitive_domain_alias`; Molten's existing macro and alias APIs remain unchanged. This observed local coverage still requires immutable-cohort adoption before acceptance.

## Core, admission, and wire APIs

`molten_core::nominal` owns the pure reference families and admitted sets. `admit_reference(ReferenceRole, &str)` is an explicit heterogeneous raw-input admission boundary; `AdmittedReference` retains typed variants. `decide_authority` compares complete typed authority sets while separately requiring policy approval, non-expiry, and non-revocation. `historical_replay_is_evidence_only` does not mint current authority.

`authority::nominal` owns these explicit compatibility boundaries:

- `AuthorityWireDto`, `ExecutionWireDto`, `ArtifactWireDto`, and `HistoricalWireDto` retain their existing `String` fields and schemas.
- `admit_authority_wire`, `admit_execution_wire`, `admit_artifact_wire`, and `admit_historical_wire` construct exact typed sets from those DTOs.
- `project_authority_wire`, `project_execution_wire`, and `project_artifact_wire` return the exact admitted text to the wire encoder.
- `admit_context_refs` produces typed context/subject/delegation/revocation/key/policy/evidence fields in `AdmittedContextRefs`.
- `admit_currentness_request` accepts explicit raw request inputs and produces typed principal/operation/current-key fields in `AdmittedCurrentnessRequest`.

The real `authority_grant_currentness` path in `src/authority/parts/mod/p001/body.rs` consumes admitted principal and key values. It still has compatibility inputs and is not declared as a migrated Octet scope by this rollout. Preserves DTOs, display labels, diagnostic strings, external protocol strings, generic wrapper storage fields, and unrelated text remain outside the selected field symbols.

`require_session(&SessionRef) -> &SessionRef` provides Rust type separation. The provider's raw-value check does not inspect borrowed `&str`/`&String` regressions; no such coverage is claimed here. Likewise, typed enum payloads exist, but tuple-field symbol matching is not claimed without compiler proof.

`newtype_invariant_bypass` currently checks `refined-newtype` rows, not these `marker-generic` wrappers; this rollout does not claim that lint protects their constructors or private storage. The provider must explicitly admit the truthful `marker-generic`/`private-fallible` catalog pair in both its Nickel and Rust validators. That coordinated schema admission does not itself prove any runtime check.

## Verification contract and immutable-cohort proof

The actual `nominal-reference-domains` Nix check runs the provider's public `cargo-octet check --output-format json`, not a declaration-only source scan. Its immutable-cohort proof must satisfy:

1. Export/typecheck the Nickel review inventory, compare its fifteen catalog rows to the provider's actual embedded catalog, and review exact symbols/scopes. This checks policy agreement, not runtime enforcement.
2. Run the pinned Octet cohort on the actual admitted core source with exact denial. A focused dependency-free harness may compile that unmodified file at the same relative physical path to isolate unrelated lint debt, but must not be reported as a full-root pass.
3. Copy the actual core source at the same physical path and mutate its existing `AuthorityReferenceSet::holder` field from `PrincipalRef` to `String`, not a shadow struct; require the real `raw_domain_value` diagnostic. Compile as a library with `cfg(test)` off: the core holder constructions are test-only, so this bounded mutation needs no constructor/projection repair. First establish that plain rustc accepts it. An unrelated type error or nonzero exit is not proof.
4. Check a type-correct local-macro alias negative using the actual `domains!` source convention, a declared alias such as `PrincipalRef`, and `String`; repair only the resulting principal admission constructor in the isolated negative copy and require `primitive_domain_alias`. Preserve the actual macro path rather than substitute an explicit-source-only fixture. Plain compiler success must precede lint rejection.
5. Run the existing positive and negative admission, roundtrip, and canonical compatibility checks without modifying historical or frozen fixtures. Run native Cairn gates, traceability checks, review, and combined checks before acceptance.

Commands for the explicitly approved focused immutable-cohort acceptance scope:

```sh
nickel typecheck config/nominal-reference-domains.ncl
nickel export --format json config/nominal-reference-domains.ncl
cargo octet check -p molten -- --lib
nix build .#checks.x86_64-linux.nominal-reference-domains --no-link -L
```

The runtime hook first compares all fifteen consumer/provider catalog rows. It copies the actual nominal source and textual part into dependency-free fixture crates at the catalogued physical path, makes the store copies writable, generates offline lockfiles, and uses separate target directories. Positive acceptance requires a `clean` producer status with zero findings. Each negative must first compile with plain rustc; the public checker must then return a nonzero result with the specified error-level lint on `crates/molten-core/src/nominal.rs`. An unrelated compiler error, warning-only producer, or cached unlinted success is insufficient.

Observed immutable-cohort invocation, without a local input override:

```sh
nix build .#checks.x86_64-linux.nominal-reference-domains \
  .#checks.x86_64-linux.inherited-tracey-debt \
  .#checks.x86_64-linux.verified-node-replication-pilot \
  --no-update-lock-file --no-link --print-out-paths -L
```

The pinned nominal check passed; its final annotated-source output is `/nix/store/rq1f0gkfdf8567kjk50zjhi9k1lr5xcw-molten-nominal-reference-domains`. All fifteen consumer/provider rows match. The unchanged source is clean; the existing `AuthorityReferenceSet::holder` String mutation produces `raw_domain_value`; the actual `domains!` alias mutation produces `primitive_domain_alias` and the resulting raw holder finding. Both negative copies compile without Octet first. Exact current diagnostics and review are retained in the [final check record](../.cairn/archive/1970-01-01-adopt-octet-nominal-domain-enforcement/evidence/2026-10-16-final-checks-and-review.json); the [original pinned proof](../.cairn/archive/1970-01-01-adopt-octet-nominal-domain-enforcement/evidence/2026-10-16-pinned-runtime-proof.json) and earlier local-override proof remain preserved with their actual source/cohort scopes.

Under the pinned cohort, default-feature checks passed sixteen selected nominal/schema tests, all five frozen fabric-boundary tests, and seven existing compile-fail doctests at their moved textual-part location. Both selected libraries passed Clippy with `-D warnings`, and the touched source passed focused Rust and Nix formatting. Exact commands, outcomes, environment observations, and the combined output are retained in the [compatibility evidence](../.cairn/archive/1970-01-01-adopt-octet-nominal-domain-enforcement/evidence/2026-10-16-runtime-compatibility.json). These remain bounded checks, not a full-workspace or optional-feature acceptance claim.

The separate diagnostic `cargo octet check -p molten-core -- --lib` exits zero but reports `warning-only`, 985 warnings, and zero errors. The [original diagnostic and gate conflict](../.cairn/archive/1970-01-01-adopt-octet-nominal-domain-enforcement/evidence/2026-10-16-full-library-gate-conflict.json) remain retained; the later owner decision explicitly separates this whole-core debt from the focused adoption gate. No warning was suppressed and no whole-core clean claim is made. Other changes retain their own acceptance requirements. Compile-fail doctests demonstrate Rust category separation only.

## Concurrent ownership and migration guidance

Artifact binding retains selection, generation, retirement, and binding semantics. Semantic-effect work retains operation descriptor/effect meaning. UCAN and Basalt retain capability, caveat, delegation, revocation, key, policy, and resource authority decisions; Valence and Cairn retain evidence/lifecycle meaning. This layer owns category syntax and local Rust type separation.

For a future explicitly owned migration: decode the existing Preserves DTO, construct the exact reference type, run normal authority/policy checks, keep typed values in the core, and project text only at an explicit wire/external boundary. Compare canonical bytes and replay results before closing the cohort. Do not wrap display text or introduce generic conversion ports.

## Non-claims

<!-- r[impl molten.authority.nominal_references.docs] -->
<!-- r[impl molten.authority.nominal_references.final_checks] -->

A typed reference proves only local category separation and checked syntax. It does not prove current authority, freshness, evidence truth, transport identity, semantic equivalence, runtime correctness, release eligibility, or migration of undeclared source scopes. Configuration and inventory presence do not prove observed lint enforcement or authorize incomplete sync/archive.
