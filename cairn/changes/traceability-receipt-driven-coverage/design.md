# Design: receipt-driven traceability coverage

## Scope

This change shifts the preferred traceability input from hand-authored coverage tuples to canonical receipt refs. It complements verification-run receipts but can also consume aggregate proof manifests, gate receipts, and release receipts that expose requirement coverage metadata.

## Derivation pipeline

The CLI shell resolves receipt refs and reads canonical artifacts. The pure core validates receipt schemas, extracts requirement id and coverage kind, derives coverage entries, sorts them deterministically, and computes the same manifest summary groups as existing traceability.

Raw coverage tuples remain compatibility-only and must be labeled in summaries. Release profiles may require receipt-backed coverage once enough receipts exist.

## Hegel RS properties

Generated receipt sets should verify stable derivation, duplicate receipt handling, stale requirement denial, positive/negative separation, and monotonic denial when a stale receipt is added.

## Non-goals

- No execution of arbitrary command strings during traceability scanning.
- No trust in paths or logs outside canonical receipt refs.
