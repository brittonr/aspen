# Nominal authority and artifact references

## Result

Molten admits selected authority and artifact strings into distinct Rust reference domains.
Preserves remains the canonical wire authority.
The Rust types prevent selected local category substitutions after admission.

## Inventory

| Wire role | Rust alias | Family | Migrated core scopes | Wire status |
|---|---|---|---|---|
| principal | `PrincipalRef` | entity | authority holder | unchanged Preserves string |
| node | `NodeRef` | entity | node control | unchanged Preserves string |
| actor | `ActorRef` | entity | handler admission | unchanged Preserves string |
| service | `ServiceRef` | entity | handler admission | unchanged Preserves string |
| session | `SessionRef` | entity | authority and effect admission | unchanged Preserves string |
| authority context | `AuthorityContextRef` | entity | authority admission | unchanged Preserves string |
| delegation | `DelegationRef` | canonical BLAKE3 | capability proofsets | unchanged Preserves string |
| revocation | `RevocationRef` | canonical BLAKE3 | capability proofsets | unchanged Preserves string |
| key | `KeyRef` | canonical BLAKE3 | authority admission | unchanged Preserves string |
| policy | `PolicyRef` | canonical BLAKE3 | authority and retention | unchanged Preserves string |
| resource | `ResourceRef` | canonical BLAKE3 | effect and node admission | unchanged Preserves string |
| evidence | `EvidenceRef` | canonical BLAKE3 | authority, provenance, and replay | unchanged Preserves string |
| artifact | `ArtifactRef` | canonical BLAKE3 | binding, retention, and cache | unchanged Preserves string |
| operation | `OperationRef` | entity | effect and artifact admission | unchanged Preserves string |
| receipt | `ReceiptRef` | canonical BLAKE3 | provenance and replay | unchanged Preserves string |

Display labels, diagnostics, external protocol strings, and unrelated text remain `String`.
They are not part of this migration.

## Core and wire boundary

`molten_core::nominal` owns the pure generic families, marker domains, aliases, parsing, role enum, admitted sets, and bounded decisions.
All stored text is private.
Callers use checked constructors, `as_str`, `domain`, or consuming conversion.

`authority::nominal` owns Preserves-facing wire DTO admission and exact projection.
Wire records keep their existing strings and schema versions.
Admission constructs typed core sets before authority, execution, artifact, retention, or replay decisions.
Projection returns the exact source text, so the existing canonical encoder receives equal fields.

Heterogeneous input uses `ReferenceRole` and `AdmittedReference`.
Known-role core APIs accept exact aliases.
A policy value cannot enter an evidence constructor unless it also satisfies the evidence role grammar and the caller explicitly selects that role.
Rust still prevents direct alias substitution.

## Validation and compile-time separation

Entity references use a bounded lowercase grammar.
Canonical references require lowercase `blake3:` content references with the exact digest length.
The constructors reject empty, oversized, malformed, unsupported-algorithm, and unknown-role values.

A compile-fail doctest passes `AuthorityContextRef` to an API that requires `SessionRef`.
Rust rejects the substitution.
Positive tests cover same-domain calls and exact wire round trips.
Negative tests cover malformed refs, role mismatch, holder drift, session drift, policy drift, expiry, revocation, and possession without policy approval.

`config/nominal-reference-domains.ncl` is the reviewed declaration for future Octet nominal-domain enforcement.
The current Octet cohort does not yet provide that policy.
The declaration is therefore an adoption input, not proof that Octet enforces it today.

## Concurrent ownership

Artifact-binding work retains artifact selection, generation, retirement, and binding semantics.
Semantic-effect work retains operation descriptor and effect meaning.
This reference layer owns category syntax and local Rust type separation only.

UCAN and Basalt retain capability, caveat, delegation, revocation, key, policy, and resource authority decisions.
Valence and Cairn retain their evidence and lifecycle meanings.

## Migration guidance

1. Decode the existing Preserves record into its wire DTO.
2. Convert each selected field through the exact typed constructor.
3. Run the normal authority or policy checks on the admitted set.
4. Keep the typed value inside the core.
5. Use `as_str` only at canonical projection or an explicit external boundary.
6. Compare canonical bytes and replay results before closing a migration cohort.

Do not wrap display text or add a generic port for reference conversion.
Reference conversion is pure internal logic.

## Non-claims

A typed reference proves only local category separation and checked syntax.
It does not prove current authority, freshness, evidence truth, transport identity, semantic equivalence, runtime correctness, or release eligibility.
