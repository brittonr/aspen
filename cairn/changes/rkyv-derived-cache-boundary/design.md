## Context

rkyv provides zero-copy Rust archives. The useful Molten shape is a local acceleration layer for derived read models, replay indexes, and large immutable cache views. The risky shape is replacing canonical Preserves artifacts, hashing rkyv bytes as stack identity, or accepting unchecked archives from disk or peers.

Molten already has canonical Preserves records, BLAKE3 content identity, retention classes, replay receipts, cache key/value DTOs, and typed storage gates. rkyv should compose with those boundaries as a shell-owned materialization.

## Design

### Data flow

```text
canonical Preserves artifact(s)
  -> canonical Preserves bytes and BLAKE3 source ref(s)
  -> pure cache-admission decision
  -> optional shell-owned rkyv archive materialization
  -> validated read-only view for local Rust callers
```

The canonical Preserves artifact remains the object named by evidence, receipts, policy, storage refs, and release bundles. The rkyv archive is a disposable sidecar whose identity is only meaningful together with the source refs, source digests, archive format profile, producer/tool version, and validation receipt.

### Archive manifest

Each derived archive sidecar should have a small canonical Preserves manifest, not a bare rkyv file as the interface. The manifest records:

- cache purpose and artifact kind;
- archive schema/profile version;
- producer tool ref and version;
- canonical source refs and BLAKE3 digests;
- expected archive byte digest;
- validation requirement and validation receipt ref;
- rebuild command or rebuilding capability marker;
- retention class such as `ephemeral cache` or `replay snapshot`.

### Functional core and shell split

The pure core decides whether a derived archive may be used from in-memory manifest facts and caller-supplied current source refs. It returns `admit`, `rebuild`, or `deny` with diagnostics. It must not read archive bytes, mmap files, inspect paths, or call rkyv.

The shell owns file discovery, mmap or byte reads, bytecheck/rkyv validation, rebuilding, cache writes, and diagnostic receipts. The shell may expose a validated archived view only after the pure decision admits the manifest and archive validation passes.

### Safety rules

- Derived archive bytes MUST NOT be used as canonical Evidence IR, cache key, policy, token, release, or storage value identity.
- rkyv archives loaded from disk, peers, or bundles MUST be validated before safe access.
- Unsafe access is allowed only behind a local trusted-writer proof or a validation receipt for the exact archive bytes.
- Missing source refs, stale source digests, version mismatches, malformed manifests, bytecheck failures, and overclaiming manifests deny or trigger rebuild before semantic reads.
- Rebuilding from canonical Preserves sources must reproduce equivalent semantic data even if archive byte layout changes.

### Test strategy

Positive tests should cover current manifest admission, successful validation, and rebuild from canonical Preserves source refs. Negative tests should cover stale source refs, wrong BLAKE3 source digest, archive byte tampering, missing validation receipt, malformed manifest, incompatible archive schema/profile, and attempts to treat a derived archive as authoritative evidence or storage.

## Non-goals

- Do not replace Preserves envelopes, receipts, evidence, policy inputs, or storage values.
- Do not require rkyv for all caches.
- Do not claim rkyv archives are stable cross-language or long-lived public artifacts.
- Do not treat bytecheck validation as domain/schema/policy validation.