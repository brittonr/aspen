# Design: Canonical Content Ref Discipline

## Overview

Molten treats canonical Preserves bytes and BLAKE3 refs as the stable identity rail. The ref discipline separates three checks that are currently conflated in several modules:

1. **Shape**: the ref names a supported algorithm and has the exact digest syntax.
2. **Canonical identity**: the ref was computed from canonical Preserves bytes or domain-separated chunk bytes.
3. **Materialization**: local storage can read the claimed bytes/value and recompute the same ref.

A shared `ContentRef`/`CanonicalRef` helper owns shape parsing and formatting. Existing `canonical_hash` remains the constructor for canonical Preserves values. Chunk-store domain-separated chunk refs remain valid BLAKE3 refs but are verified by chunk manifests/readback rather than by Preserves parsing.

## Ref parser

The parser accepts only `blake3:<64 lowercase hex chars>` for canonical content refs. It rejects empty suffixes, short fixture refs, non-hex characters, uppercase digests, path separators, and future algorithms until those algorithms are explicitly modeled. Callers that need legacy fixture refs must keep them in test-only helpers or convert them to canonical fixture values.

## Materialized verification

Operations that claim a local artifact or payload is present must perform readback through the owning store:

- evidence ledger artifacts read canonical bytes and recompute `canonical_hash`,
- chunk manifests read chunks/manifests and recompute manifest/chunk refs,
- node-control live ingress reads canonical envelope bytes and binds envelope/payload/control refs,
- runtime reports/journals read canonical Preserves values and recompute state/event refs.

A well-shaped ref is never authority, policy, provenance, transport, source-gate, or retention trust. Missing or tampered materialized refs produce deny diagnostics/receipts at the operation boundary.

## Migration strategy

Migrate validators by boundary priority:

1. node-control request/live ingress payload refs and receipt refs,
2. runtime values, turn journals, state snapshots, and harness reports,
3. artifact registry/catalog/evidence ledger import/export readback,
4. coordination, protocol session, service runtime, transcripts, provenance, redaction, and secrets.

Each migration keeps existing record shapes where possible and strengthens parse/readback behavior with negative tests.
