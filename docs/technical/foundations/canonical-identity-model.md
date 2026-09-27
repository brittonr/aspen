# Canonical Identity Model

Molten's identity boundary turns a semantic value into stable canonical bytes and then a content reference. It does not turn a Rust allocation, transport endpoint, or human-readable rendering into authority. This article assumes basic familiarity with content addressing and explains the inspected codec pipeline, its validation layers, and several easily confused meanings of “valid.” The [Technical companion](../README.md) supplies the wider navigation context.

## Identity has a representation boundary

The [architecture](../../architecture.md#core-envelope-spine) specifies canonical Preserves bytes with BLAKE3 identity. Preserves records carry boundary values such as envelopes, policy decisions, receipts, and identity-bearing durable records. Rust structures remain in-memory inputs and outputs; their field layout and debug formatting are not canonical definitions.

Conceptually, identification follows `value -> canonical Preserves bytes -> BLAKE3 reference`. This is an explanatory composition, not a new API. The actual `canonical_bytes` helper calls `preserves::write_iovalue_packed(value, false)`. `canonical_hash` invokes that helper and passes its result to `content_ref_from_bytes`, which hashes the bytes with BLAKE3 and formats the reference. The [byte encoder](../../../src/preserves/parts/rail/p001/body.rs) and [hash helpers](../../../src/preserves/parts/rail/p003/body.rs) therefore distinguish canonical-value hashing from hashing an arbitrary byte buffer.

That distinction is important: `content_ref_from_bytes` is not itself a canonicality checker. A digest can identify arbitrary bytes perfectly well without making those bytes an accepted canonical envelope. Callers need the appropriate boundary operation rather than treating every BLAKE3-shaped string as equivalent evidence.

## Four checks with different conclusions

First, a reference parser can establish lexical validity. In the shared codec, `validate_content_ref` requires the `blake3:` prefix and 64 lowercase hexadecimal characters. This does not fetch the referenced object or verify that any available object's bytes match it.

Second, `strict_canonical_decode` decodes packed Preserves, re-encodes the value canonically, and compares the re-encoding with the entire supplied byte slice. If the bytes differ, it denies rather than silently normalizing an identity-bearing input. On success, it returns the value, canonical bytes, and computed reference. This comparison distinguishes “can be parsed” from “is already the required representation.”

Third, `strict_canonical_decode_with_ref` parses an expected content reference, performs strict decoding, and compares the computed reference with the expected one. Canonicality and reference agreement are separate obligations: canonical bytes can still be the wrong object.

Fourth, domain validation checks interpretation. The pure `validate_domain_artifact` inspects a domain name, supported label, schema equality, and reference shape, returning a `DomainArtifactSummary`. It neither parses Preserves nor recomputes the digest because `DomainArtifactInput` supplies no bytes. The [modularity inventory](../../modularity-boundaries.md#domain-codec-façade) describes it as a domain-owned check after canonical ref computation, not a replacement for byte verification. Its precise code is in the [core codec](../../../crates/molten-core/src/codec.rs).

## Worked failure scenario: authentic bytes, wrong meaning

Consider an illustrative receiver expecting a chunk manifest under its current schema. It receives a canonical record with a correct digest, but the record identifies an unsupported manifest label. Strict decoding and reference matching can succeed while domain validation rejects `UnsupportedLabel`. No contradiction exists: the first stages identify bytes and their value; the final stage checks whether that value belongs to the consumer's admitted contract.

A different sender might transmit a parsable noncanonical encoding of the expected value. Silently decoding and re-encoding it before verification would lose the fact that the received bytes were outside the required boundary. The strict helper instead compares the original slice with its canonical reconstruction. This makes acceptance a statement about the actual input representation, not merely an equivalent value recovered from it.

Even success at all four stages does not authorize a destructive operation using the object. Identity answers which artifact is under discussion. Authority and policy answer what an actor may do with it. The [fabric boundary](../../distributed-system-fabric.md#canonical-identity-and-adapters) explicitly excludes paths, backend ids, tickets, and transport frames from canonical authority.

## Review guidance and a scoped discrepancy

During review, identify which helper a caller uses and which conclusion the caller draws. Search for a concrete expected reference comparison, then inspect the label/schema check and subsequent admission. Do not substitute a filename containing a digest for measured bytes. For large referenced payloads, distinguish the metadata record's identity from the payload's own content reference and availability.

There is a narrow implementation difference worth preserving in review: the pure core codec's `valid_blake3_ref` accepts ASCII hexadecimal characters, including uppercase, whereas the shared codec requires lowercase. Thus a core summary's shape acceptance does not prove acceptance by the shared `ContentRef` boundary. This article records the two inspected predicates rather than declaring their accepted languages identical or inventing a normalization rule. No code or governing policy is changed here.

Suggested verification includes noncanonical input rejection, mismatched expected refs, unsupported labels, and schema drift. The core codec already contains domain-validation tests. These are review targets, not tests executed for this article.

## Limits and non-claims

Content addressing does not prove provenance, authorization, durability, availability, semantic correctness, or release readiness. A canonical receipt can faithfully identify a denial or an incomplete observation. Hash identity is not Rust-layout identity, and successful decoding is not permission to execute. The governing documents retain authority over schema and admission requirements.

## Sources

- [Architecture: canonical envelope spine](../../architecture.md)
- [Fabric canonical identity boundary](../../distributed-system-fabric.md)
- [Modularity: domain codec façade](../../modularity-boundaries.md)
- [Reference parsing and strict canonical decoding](../../../src/preserves/parts/rail/p001/body.rs)
- [Canonical hashing and expected-ref checks](../../../src/preserves/parts/rail/p003/body.rs)
- [Pure domain codec and tests](../../../crates/molten-core/src/codec.rs)
