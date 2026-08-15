## Context

Molten already identifies many objects by canonical hash. Large values may live in Iroh blobs or another store and be referenced from Preserves metadata. To make this scalable, Molten needs a chunk-store layer that works below artifact/storage/replay/job abstractions while preserving their canonical identity and evidence.

The chunk store does not define application semantics. It provides deterministic chunking, chunk verification, object manifests, deduplication, resumable fetch, range reads, retention pins, and evidence for large immutable byte sequences.

## Central rule

Large canonical objects SHOULD be represented by a deterministic chunk manifest:

```text
object_ref = hash(
  domain_separator,
  object_kind,
  chunker_version,
  total_len,
  root_or_chunk_refs,
  canonical_metadata
)
```

Chunks are content-addressed independently. Object identity is the manifest/root identity, not a mutable filename or transport-specific blob id.

## Goals

- Chunk large canonical byte sequences deterministically.
- Support chunk-level deduplication across Molten object kinds.
- Verify chunks and manifests incrementally during streaming fetch.
- Support resumable fetch and range reads where manifests permit it.
- Preserve deterministic replay by pinning chunker version and manifest metadata.
- Track chunk-level retention pins and safe GC eligibility.
- Support encryption and redaction without undermining commitments or replay evidence.
- Keep Iroh/Redb as adapters below canonical chunk manifest semantics.

## Non-Goals

- Do not replace Preserves canonical object identity for small values.
- Do not make transport-specific blob ids authoritative Molten identity.
- Do not use nondeterministic or implementation-dependent chunking.
- Do not delete chunks reachable from pinned manifests.
- Do not leak plaintext through chunk hashes when confidentiality policy requires protected commitments.

## Chunking modes

Initial mode:

- `fixed_v1`: fixed-size chunks with explicit size, deterministic final chunk, domain-separated chunk hashes.

Future mode:

- `content_defined_vN`: content-defined chunking for better dedup, only after the algorithm, parameters, and chunker version are pinned and reproducible.

The chunker version and parameters are part of the manifest. Changing chunking changes object refs unless a higher-level canonical object ref explicitly abstracts over equivalent chunk layouts.

## Chunk refs

A chunk ref should include:

- chunk hash,
- chunk length,
- chunker version/domain,
- optional compression/encryption metadata refs,
- storage location hints such as Iroh blob ticket or local store key,
- evidence refs for verification/fetch where needed.

Location hints are not identity.

## Manifests

A manifest should include:

- object kind and domain separator,
- total byte length,
- chunker version and parameters,
- ordered chunk refs or Merkle tree root with proof scheme,
- canonical metadata hash,
- compression/encryption/redaction policy refs,
- schema/artifact/storage/replay refs where relevant,
- evidence refs and pin refs.

Small objects may be inline Preserves values. Large objects carry a manifest ref from the enclosing Preserves value.

## Streaming verification

Fetch should verify:

1. manifest hash/root,
2. each chunk hash and length as it arrives,
3. chunk order/proof against the manifest,
4. reconstructed total length and optional root hash,
5. object-level schema/policy/admission after integrity succeeds.

Receipts should record manifest id, chunk refs fetched, missing chunks, verification results, and denial reasons.

## Range reads and resumable fetch

Fixed-size manifests support deterministic byte-range to chunk mapping. A fetch can resume by requesting only missing chunks. Range reads should verify the relevant chunks and proofs before exposing bytes. Partial fetch state is cache metadata, not object identity.

## Deduplication

Chunks may be shared across:

- artifact payloads,
- Iroh blob payloads,
- typed storage large values,
- snapshots and replay logs,
- trace archives,
- transcript outputs,
- docs/media,
- distributed job input partitions and intermediate outputs.

Dedup must respect confidentiality policy. Two ciphertext chunks may dedup if encryption mode permits; plaintext dedup may be denied for sensitive data.

## Compression and encryption

Ordering matters and must be explicit:

- `compress_then_chunk`
- `chunk_then_compress`
- `encrypt_then_chunk`
- `chunk_then_encrypt`

For confidentiality, chunk hashes over plaintext may leak equality. Secret-bearing objects should use encrypted chunk refs or safe commitments according to policy. Replay can compare commitments when plaintext reveal is denied.

## Retention and GC

Pins can attach to object manifests or individual chunks. A chunk is GC-eligible only if no pinned or retained manifest can reach it and no partial fetch/cache policy retains it. If reachability proof is incomplete, deny deletion. Deletion emits tombstone/GC receipts.

## Adapter mapping

Redb stores chunk indexes, manifests, pin metadata, and local availability. Iroh blobs can store chunks, manifests, or packed chunk groups. Iroh docs may publish availability hints or manifest metadata. Adapter locations are mutable hints, not identity.

## Integration points

Remote artifact sync uses manifests to compute missing chunks and resume fetches.

Typed storage stores large values as manifest refs and validates chunks before load.

Deterministic replay fetches only needed snapshot/log chunks and includes manifest refs in replay identity.

Evaluation cache can cache validation over manifest refs and chunk proof results.

Retention/GC uses manifest reachability and chunk pins.

Catalog/MCP can inspect manifests, chunk availability, dedup ratios, and GC pins subject to visibility policy.

## Open Questions

- What fixed chunk size should be the first default?
- Should Merkle trees be mandatory immediately or are ordered chunk lists sufficient for the first milestone?
- Which encryption envelope should be paired with chunk refs first?
- Should Iroh blobs store one chunk per blob or packed chunk groups for efficiency?
