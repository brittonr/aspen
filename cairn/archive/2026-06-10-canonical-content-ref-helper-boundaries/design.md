# Design: Canonical Content-Ref Helper Boundaries

## Shared construction helpers

Canonical content refs are constructed at one boundary: the Preserves rail helper layer. Callers that have raw bytes, an already computed BLAKE3 hash, or a lowercase hex digest use the shared helper variants instead of hand-building `blake3:` strings.

The hex-based helper validates the digest before returning a ref. This makes filename/readback conversion from `blake3_<hex>.bin` fail closed for malformed names rather than synthesizing a plausible ref string.

## Materialized readback

Shape validation remains separate from materialized content availability. Ledger, chunk-store, ingress, and transport stores still recompute refs from bytes or canonical values before treating local content as present. The helper only provides canonical formatting and parsing; it does not prove local availability or authority.

## Transitional external aliases

Some external tools use alternate short hash labels, currently Octet object-corpus and fingerprint evidence with `b3:`. Those labels are diagnostic/evidence-specific aliases, not runtime content refs. Any conversion to `b3:` is local to the integration and derived from a validated canonical `blake3:` ref or equivalent checked helper path.

## Evidence and trust boundary

A correctly formatted ref is identity evidence only. Downstream operations still require their existing authority, policy, provenance, source-gate, retention, resource, transport, and replay gates before side effects.
