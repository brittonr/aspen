# Proposal: Canonical Content Ref Discipline

## Why

Molten already uses BLAKE3 content references throughout receipts, ledgers, chunks, jobs, node control, and runtime evidence. Many boundaries still validate refs with local string checks such as `starts_with("blake3:")`, which proves only shape and not canonical identity, materialization, or readback. This weakens fail-closed behavior at exactly the boundaries where Molten wants content addressing to be the stable identity rail.

## What Changes

- Introduce a shared canonical content-ref type and parser for `blake3:<64 lowercase hex chars>` refs.
- Replace ad-hoc ref validators with a common helper that rejects malformed, truncated, non-hex, or unsupported-algorithm refs.
- Distinguish ref shape validation from materialized content verification; a well-shaped ref does not grant trust.
- Require local readback verification where an operation claims a local artifact, payload, receipt, envelope, or chunk manifest is present.
- Emit deny receipts/diagnostics for missing, stale, or tampered materialized refs instead of accepting plausible-looking strings.
- Keep BLAKE3 content addressing as evidence identity only; policy, authority, provenance, source gates, retention, and transport admission remain separate gates.

## Impact

The first implementation slice can add the shared ref type, migrate high-value node-control and runtime-boundary validators, and add negative tests for malformed and missing refs. Later slices can migrate artifact registry, protocol, coordination, service, transcript, redaction, and provenance validators without changing receipt schemas unnecessarily.
