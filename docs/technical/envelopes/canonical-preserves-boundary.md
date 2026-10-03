# Canonical Preserves Boundary

This article explains what Molten identifies when it hashes a boundary value, and why decoding a value is weaker than admitting its original bytes. It assumes familiarity with Preserves records and content addressing. The [architecture](../../architecture.md#core-envelope-spine) remains authoritative; this is a [Technical companion](../README.md), not an additional wire specification.

## Identity is a projection, not a memory image

The stable object is the explicitly constructed Preserves value. For runtime envelopes, `Envelope::to_value` constructs a `runtime-envelope-v1` record containing the version, sender, subject, body, blob-reference sequence, capability sequence, and evidence-reference sequence. `canonical_bytes` and `canonical_hash` operate on that projection, not on the Rust struct, its serializer metadata, pointer layout, or debug representation. See the [envelope implementation](../../../src/runtime/envelope/mod.rs).

This distinction permits an internal refactor without necessarily changing wire identity. Conversely, changing a projected field can change identity even if the Rust type remains unchanged. “Same type” and “same boundary value” answer different questions. A DTO's JSON representation is likewise not the envelope's canonical representation: the DTO carries textual subject and body values that are parsed before constructing the envelope.

The shared rail's `canonical_bytes` calls `preserves::write_iovalue_packed(value, false)`. `canonical_hash` hashes those bytes through the content-reference helper. Thus the useful reasoning chain is explicit value construction, canonical packed encoding, then BLAKE3 reference—not arbitrary serialization followed by a hash. The [encoding implementation](../../../src/preserves/parts/rail/p001/body.rs) and [hash helpers](../../../src/preserves/parts/rail/p003/body.rs) are the concrete boundary.

## Strict decoding preserves the byte-level claim

A parser answers whether bytes can be interpreted. `strict_canonical_decode` additionally re-encodes the parsed value and compares the resulting byte slice with the original input. A mismatch is an error. Successful decoding returns the value, its canonical bytes, and its content reference. `strict_canonical_decode_with_ref` then compares that computed reference with a checked expected reference.

These checks protect two distinct assertions:

1. The incoming bytes themselves are the canonical representation, rather than merely a representation that can be normalized.
2. The canonical representation is the particular object the caller expected.

Neither assertion establishes that the value belongs to an application schema. A canonical integer, for example, is still not a correctly shaped receipt. The rail handles schema validation separately. Nor does successful canonical decoding establish sender identity, current capability authority, or whether referenced evidence is truthful.

The distinction between normalization and rejection matters at durable or evidence-bearing boundaries. Silently accepting a noncanonical representation and reporting only its normalized hash would lose the relationship between received bytes and the identity being discussed. Text import can legitimately parse a human representation and construct a canonical value; strict binary ingress makes a stronger, different claim.

## Worked failure: equivalent meaning, inadmissible bytes

Consider an illustrative receiver expecting a canonical packed representation of `<strict-decode-fixture "payload">`. A sender supplies parseable bytes containing an annotation that is not retained by the selected canonical encoding. Parsing may recover the intended value, but strict decoding compares the original with the annotation-free canonical re-encoding and rejects the mismatch.

Now consider a different attack: an envelope is changed and freshly encoded canonically, while its old declared reference is retained. Canonicality alone cannot reject a different well-formed value. The expected-reference comparison supplies the missing condition. These are different failures and deserve different review questions: “Were these bytes canonical?” versus “Were these the expected bytes?”

The existing [strict-decode tests](../../../src/preserves/parts/rail/tests/m000/p000/body.rs) cover canonical acceptance, annotated input, trailing bytes, truncation, and tampering against an expected reference. They are source evidence for those cases, not a claim that this documentation change executed them.

## Reviewing boundary changes

For an encoder change, identify the exact Preserves projection before looking at Rust derives. Inspect field order, record labels, optional-value representations, and any sequence construction. Do not assume that application-level set semantics imply that an emitted sequence is order-insensitive.

For a decoder change, determine whether the entry point imports human text, accepts arbitrary parsed values, or requires canonical packed bytes. Review whether a declared content reference is actually compared with a recomputed reference. For a schema-bearing boundary, follow the additional schema check rather than inferring it from successful parsing.

Suggested verification is to compare canonical bytes before and after a DTO round trip, then independently exercise malformed packed input and a valid canonical value with the wrong expected reference. The [envelope tests](../../../src/runtime/envelope/tests.rs) already express the round-trip and equivalent-value cases. No tests were run for this article.

## Limits and non-claims

Canonical identity is not authorization, a signature, a retention guarantee, or a proof of semantic correctness. Hash agreement does not make a remote transport trusted. A deterministic boundary helper does not execute filesystem, network, clock, or entropy effects, and its result does not establish production readiness. Schema identity, typed-reference categories, and policy admission add different checks; none can be collapsed into the byte-identity claim.

## Sources

- [Architecture and envelope spine](../../architecture.md#core-envelope-spine)
- [Nominal reference boundary and non-claims](../../nominal-authority-references.md)
- [Runtime envelope projection](../../../src/runtime/envelope/mod.rs)
- [Canonical encoding and strict decoding](../../../src/preserves/parts/rail/p001/body.rs)
- [Expected-reference checks and hashing](../../../src/preserves/parts/rail/p003/body.rs)
- [Strict-decode regression cases](../../../src/preserves/parts/rail/tests/m000/p000/body.rs)
