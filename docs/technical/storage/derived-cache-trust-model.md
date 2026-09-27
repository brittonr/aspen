# Derived-cache trust model

A derived cache can accelerate access without becoming the authority for the value it represents. Molten's rkyv boundary makes this distinction explicit: canonical Preserves values and BLAKE3 source refs retain identity, while an archive is a rebuildable sidecar. This article assumes the [rkyv derived-cache contract](../../rkyv-derived-cache-boundary.md), examines the inspected manifest and admission functions, and belongs to the [Technical companion](../README.md).

## Two identities answer different questions

The canonical source identifies the value or artifact that other subsystems mean. The archive byte digest identifies one concrete representation used for local access. A producer upgrade can change archive bytes without changing the canonical value. Conversely, unchanged archive bytes can be stale because the current canonical source has changed. Equating these identities would confuse representation freshness with artifact identity.

`RkyvSourceDigest` records a source ref and a BLAKE3 digest. In the [manifest implementation](../../../src/eval/parts/cache/p007/body.rs), `rkyv_source_digest` validates the ref and computes `canonical_hash` over the supplied canonical value. The archive manifest then records purpose, artifact kind, profile, producer tool and version, source digests, archive digest, validation requirements and receipt, rebuild capability, retention class, and identity claim. Its own `manifest_ref` comes from hashing the constructed Preserves value.

This does not make archive layout canonical. The explicit accepted identity claim is `derived-sidecar`. Producer metadata describes how the sidecar was produced; it is not a substitute for observing current source refs and archive bytes.

## Admission consumes facts; shells obtain them

`RkyvArchiveAdmissionInput` contains the manifest, current sources, observed archive digest, observed validation receipt ref, validation result, and whether the caller allows rebuilding. `admit_rkyv_derived_archive` validates observation shape, collects diagnostics, selects a decision, and constructs a Preserves admission value. No byte read, memory mapping, archive traversal, or rebuild occurs in this function.

The [governing contract](../../rkyv-derived-cache-boundary.md) assigns byte acquisition, mmap, bytecheck/rkyv validation, rebuilding, and cache writes to the shell. “Pure before shell I/O” is therefore an architectural separation of decision from effect, not a claim that an observed byte digest can exist without any prior read. The pure function evaluates supplied facts before admitting cache use or selecting subsequent effects; it cannot independently attest that the shell measured the intended bytes.

Nor does this inspection establish a complete production mmap pipeline. The concrete code reviewed here implements manifest construction and fact-based admission in the cache module. A reader assessing a particular archive consumer still needs to inspect that consumer's byte acquisition and validation boundary rather than infer it from these types.

## Decision precedence is deliberately asymmetric

The [diagnostic implementation](../../../src/eval/parts/cache/p008/body.rs) checks identity overclaim, unsupported profile, missing sources, stale source refs or digests, archive-digest mismatch, required validation failure, and required validation-receipt mismatch. Source comparison treats pairs of ref and digest as a set after checking equal lengths; source validation rejects duplicate refs and enforces a finite count. Source-list order is consequently not a freshness distinction in this admission check.

No diagnostics means `admit`. Otherwise, `rebuild` requires both caller permission and a present rebuild capability, and every diagnostic must be classified rebuildable. The current implementation classifies diagnostic text containing `stale` or `byte digest` as rebuildable. This is the observed mechanism, not a general rule that every cache problem can be repaired automatically.

Missing required validation, unsupported profiles, and canonical-identity overclaims are not normalized into cache misses. They lead to `deny` unless the collected diagnostics satisfy the narrower rebuild rule, which those diagnostic classes do not. The boolean `validation_required` also matters: the inspected checks of validation success and receipt equality are conditional on it. An unconditional assertion that every constructed manifest demands validation would exceed this implementation.

## Illustrative stale replay index

Suppose an illustrative replay-index sidecar records source ref `S` with digest `D1`, archive digest `A1`, and a matching validation receipt. The current canonical observation supplies the same source ref with digest `D2`. Even if the archive remains byte-for-byte `A1` and its old byte validation passed, source freshness fails. With caller permission and a rebuild capability, the admission result requests `rebuild`; it does not authorize using stale index entries while rebuilding.

Now add an observed validation failure where validation is required. There are two diagnostics: staleness and failed validation. Because all diagnostics must be rebuildable, the second prevents a rebuild decision in this implementation. Treating “rebuildable cache” as a blanket fallback would silently erase that distinction.

Finally, changing the manifest's claim to canonical identity creates a different category of failure. The archive is attempting to become the source of truth. Rebuilding its bytes cannot cure that authority overclaim; the caller must not reinterpret a denied sidecar as a durable canonical value.

## Verification guidance and limits

Suggested review cases are current validated admission, stale sources with and without rebuild permission, archive tampering, combined stale-plus-validation failure, unsupported profiles, duplicate source refs, and identity overclaims. The [existing cache tests](../../../src/eval/parts/cache/tests/m000/p002/body.rs) cover admission, stale or tampered archives, validation, profile, and identity denial. They are cited as source evidence, not reported as executed here.

Typed storage still obtains durable value identity, schema conformance, migration inputs, and release or evidence refs from canonical Preserves values. A derived cache admission does not confer read authority, retention authority, deletion permission, or release readiness. Likewise, archive byte validation is not a proof of producer trust, semantic equivalence of a faulty rebuild, freshness after arbitrary concurrent mutation, or crash durability. The [content-store boundary](../../content-store-adapter.md) makes the parallel distinction between verified content observations and separate authority decisions.

## Sources

- [rkyv derived-cache boundary](../../rkyv-derived-cache-boundary.md)
- [Content-store adapter runtime](../../content-store-adapter.md)
- [Archive manifest and admission inputs](../../../src/eval/parts/cache/p007/body.rs)
- [Admission diagnostics, rebuild precedence, and canonical records](../../../src/eval/parts/cache/p008/body.rs)
- [Derived-cache regression scenarios](../../../src/eval/parts/cache/tests/m000/p002/body.rs)
- [Technical companion](../README.md)
