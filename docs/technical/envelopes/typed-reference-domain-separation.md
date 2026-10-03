# Typed Reference Domain Separation

Many authority and artifact references share a string representation while serving different roles. Molten's nominal reference layer preserves that wire format but makes selected local substitutions type errors. This article assumes Rust generics and the canonical boundary model. The [nominal reference guide](../../nominal-authority-references.md) is authoritative; this [Technical companion](../README.md) explains its implementation and limits.

## Two families, many domains

The pure core defines `EntityRef<D>` and `CanonicalRef<D>`, each with private stored text and a `PhantomData<D>` marker. `ReferenceDomain` supplies a domain name. Domain aliases include `SessionRef`, `AuthorityContextRef`, `PolicyRef`, `EvidenceRef`, and `ArtifactRef`, among others. Because their marker types differ, aliases from different domains remain different Rust types even when their text is identical. See the [core implementation](../../../crates/molten-core/src/nominal.rs).

Entity references accept nonempty strings of at most 256 bytes, using lowercase ASCII letters, digits, and the separators `-`, `_`, `.`, `:`, and `/`. A separator cannot begin or end the value. Canonical references require `blake3:` plus exactly 64 lowercase hexadecimal digits. These constructors perform syntax checks without IO, clocks, evidence loading, or policy evaluation.

The constructor does not require a textual prefix corresponding to its domain name. For example, entity syntax alone does not prove that a value denotes an existing session. The marker says which role the caller explicitly selected and which APIs can consume the resulting value; it does not infer that role from an external registry.

## Domain separation is not a new hash scheme

Here, “domain separation” means local type separation. It does not mean that `PolicyRef::new` and `EvidenceRef::new` apply different hash prefixes to the same object. Both canonical-reference families check the same content-reference grammar. A caller can explicitly parse the same well-formed digest string into both domains.

The protection is against accidental substitution after admission. An API requiring `SessionRef` cannot accept `AuthorityContextRef` directly. To cross categories, code must return to text and explicitly select another constructor or role. That is a visible review point rather than an invisible implicit conversion. It remains possible for deliberately incorrect code to choose the wrong constructor; Rust cannot infer application meaning from a hash string.

This is consistent with the [architecture](../../architecture.md#fabric-ownership): canonical Preserves values, not Rust layout, define boundary identity. Marker types do not add hidden fields to the canonical record and do not make the in-memory representation authoritative.

## Wire admission and exact projection

The [authority adapter](../../../src/authority/nominal.rs) keeps wire DTO fields as strings. `admit_authority_wire` maps holder, session, context, delegation, revocation, key, policy, resource, and evidence into their exact aliases. Related functions admit execution, artifact, and historical sets. Projection functions recover the same source text through `as_str`.

That pattern supports a wire-compatible migration: decode the existing boundary, construct selected typed fields, retain types during core decisions, then project exact text when rebuilding the canonical value. It does not require a new schema version solely because a private Rust representation became typed. The compatibility claim still depends on preserving the full projection, not just on using nominal wrappers.

For heterogeneous inputs, `ReferenceRole` and `AdmittedReference` expose role selection explicitly. `ReferenceRole::parse` rejects unknown names; `admit_reference` dispatches to the matching checked constructor. Known-role APIs can instead require the precise alias, avoiding a runtime role branch where the signature already knows the category.

## Worked category-confusion scenario

Suppose, illustratively, a function that checks a session receives an authority-context reference because two adjacent string arguments were exchanged. Before typed admission, both values can be valid strings, so ordinary type checking cannot distinguish them. With a signature taking `SessionRef`, passing `AuthorityContextRef` is rejected at compile time. The core's compile-fail doctest captures that class of substitution.

A second scenario is subtler. An evidence digest is deliberately supplied to `PolicyRef::new`. Its syntax can pass because both roles use canonical-reference grammar. The wrapper does not inspect the referenced artifact. Policy loading, schema checks, applicability, freshness, and actual approval remain necessary. Typed admission narrows one class of accidental programming errors without replacing semantic authority checks.

## Scope boundaries that matter in review

The reference guide describes selected migrated scopes, not every string-shaped type in the repository. In particular, the [runtime envelope module](../../../src/runtime/envelope/mod.rs) has its own `ActorId`, `Capability`, and `EvidenceRef` wrappers. Their names and constructors do not establish that they are the same types or have the same checks as the core nominal aliases. Review imports and physical definitions rather than matching names in isolation.

Suggested verification follows the governing migration guidance: positive same-domain calls, negative cross-domain compile checks, malformed-input rejection, exact wire projection, and comparison of canonical bytes across the migration. Inspect the existing core tests and authority round-trip tests for those local claims. No tests were executed for this article.

## Limits and non-claims

A typed reference proves checked syntax and local category separation only. It does not prove possession, current authority, revocation status, transport identity, truth of evidence, semantic equivalence, or release eligibility. The [reviewed domain declaration](../../../config/nominal-reference-domains.ncl) is an adoption input; the governing guide explicitly says the current Octet cohort does not yet enforce that policy. Neither configuration presence nor a successful constructor closes that enforcement gap.

## Sources

- [Nominal authority and artifact references](../../nominal-authority-references.md)
- [Architecture and canonical authority](../../architecture.md#fabric-ownership)
- [Pure nominal families, constructors, and tests](../../../crates/molten-core/src/nominal.rs)
- [Wire admission and projection](../../../src/authority/nominal.rs)
- [Distinct runtime envelope wrappers](../../../src/runtime/envelope/mod.rs)
- [Nominal-domain adoption declaration](../../../config/nominal-reference-domains.ncl)
