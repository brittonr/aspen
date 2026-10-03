# Content References Versus Inline Values

An envelope can carry a value directly and identify other bytes indirectly. Understanding that split is essential for reviewing integrity, availability, replay, and authority claims. This article assumes basic content addressing and follows the [architecture's envelope spine](../../architecture.md#core-envelope-spine). It is a [Technical companion](../README.md), not a storage or transport specification.

## What an envelope actually includes

`Envelope` holds its subject and body as `RuntimeValue` instances. Its canonical projection embeds their Preserves values in `subject` and `body` records. By contrast, `blob_refs` is a sequence of content-reference strings; the referenced payload bytes are not embedded by `Envelope::to_value`. Capability and evidence lists have their own fields. See the [envelope implementation](../../../src/runtime/envelope/mod.rs).

The resulting envelope hash binds the inline values and the exact projected reference sequence. It does not hash fetched blob bytes at envelope-encoding time. A changed blob reference changes the envelope value, but unavailable bytes do not prevent this pure function from hashing a well-formed reference-bearing envelope.

`Envelope::boundary` exposes an envelope reference, subject reference, body reference, blob references, and evidence references. These are useful indexes into distinct identity questions. Equal body references do not imply equal envelopes: a sender, subject, capability sequence, or referenced blob can differ. Likewise, equal envelopes identify the same boundary value but do not establish that every external dependency is locally present.

The [architecture](../../architecture.md#core-envelope-spine) recommends Preserves metadata and content references for large payloads, whose bytes may live in Iroh blobs or another store. It does not specify a universal byte threshold at which a value must be externalized. No such threshold is introduced here.

## Syntax, integrity, and availability

`ContentRef::parse` accepts the `blake3:` prefix followed by exactly 64 lowercase hexadecimal characters. That constructor verifies syntax; it does not contact a store or verify an object. The [reference implementation](../../../src/preserves/parts/rail/p001/body.rs) makes this a deterministic local operation.

`verify_blob_reference` adds a different check: given already available bytes and a declared reference, it calculates their content reference and compares the result. Success returns `BlobReferenceRecord` with the verified reference and byte length. It does not fetch the bytes, establish their retention period, or authorize a consumer to read them. These mechanics appear in the [runtime bridge](../../../src/runtime/bridge/mod.rs).

The hash input also matters. `canonical_hash` first encodes a Preserves value canonically; `verify_blob_reference` hashes the supplied raw bytes. A blob is not implicitly a Preserves value. If a subsystem promises that a referenced object contains canonical Preserves, that subsystem needs the corresponding canonical decode and schema checks in addition to byte integrity. A shared `blake3:` spelling does not identify the encoding contract by itself.

## Metadata describes; it does not prove

Consider an illustrative body containing a media label and a claimed payload length, accompanied by one blob reference. The envelope binds those claims as metadata. `verify_blob_reference` can establish the actual length of the supplied bytes and their digest, but the envelope constructor does not compare arbitrary body metadata with that result. A consuming schema or application contract owns that relationship.

The same caution applies to evidence references. Their presence binds which references the sender included; it does not establish that the evidence exists, is applicable, is current, or supports the sender's conclusion. The [nominal reference guide](../../nominal-authority-references.md#non-claims) explicitly separates checked reference syntax from evidence truth and current authority.

## Worked failure: correct envelope, wrong blob

Suppose, illustratively, an envelope correctly names a blob containing `blob payload`. The receiver checks the envelope's declared reference with `admit_remote_envelope`, which recomputes the envelope hash and returns the listed blob references. A later store response supplies `tampered` under that requested reference.

The envelope check can pass because the envelope itself has not changed. The separate `verify_blob_reference` comparison rejects the store response because the supplied bytes hash differently. The bridge's existing tampering test exercises this distinction. Conversely, if no store response arrives, there is no byte-integrity result at all: absence is an availability problem, not evidence that the envelope hash was wrong.

This separation explains why reference-bearing envelopes are useful but not sufficient for deterministic playback. The [architecture](../../architecture.md#core-envelope-spine) binds playback to artifacts, dependency closure, initial state, schemas, policies, handler profile, and seed or recorded effects. A top-level envelope reference does not materialize that closure or replace those additional inputs.

## Verification and review guidance

Suggested review starts by classifying every payload field: inline canonical value, raw-byte content reference, evidence reference, or authority-bearing input interpreted elsewhere. Trace where referenced bytes are obtained and where the digest comparison occurs. Check whether a Preserves interpretation requires a further strict decode. Do not assume a transport topic or store key is a substitute for verified content identity.

Useful verification scenarios include matching and mismatching blob bytes, unavailable referenced content, identical inline bodies with different blob lists, and metadata inconsistent with verified bytes. The inspected [bridge tests](../../../src/runtime/bridge/mod.rs) cover digest tampering and envelope-reference mismatch; they were not run for this article. The other scenarios are review suggestions, not claims of existing coverage.

## Limits and non-claims

Content addressing establishes identity relationships, not confidentiality, publication rights, garbage-collection safety, freshness, or eventual delivery. It does not turn an integrity record into an authority receipt. Pure envelope and verification helpers consume in-memory inputs; actual storage and transport effects belong to admitted adapters. No exactly-once, durability, or production-readiness claim follows from a matching digest.

## Sources

- [Architecture: inline bodies, content references, and playback](../../architecture.md#core-envelope-spine)
- [Nominal reference non-claims](../../nominal-authority-references.md#non-claims)
- [Envelope and boundary projection](../../../src/runtime/envelope/mod.rs)
- [Content-reference grammar](../../../src/preserves/parts/rail/p001/body.rs)
- [Canonical-value hash helpers](../../../src/preserves/parts/rail/p003/body.rs)
- [Blob verification and bridge tests](../../../src/runtime/bridge/mod.rs)
