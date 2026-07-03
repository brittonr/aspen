# Design: proof review readbacks

## Scope

Readbacks are deterministic rendered views over canonical proof artifacts. They are not normative evidence and cannot override a canonical deny receipt.

## Readback shape

A requirement readback should group evidence by requirement id and include positive receipts, negative receipts, aggregate proof manifests, verification-run receipts, artifact refs, stale diagnostics, exemptions, and explicit out-of-scope caveats.

The pure core builds an in-memory readback DTO from validated traceability and proof manifests. The CLI shell renders text, markdown, or Preserves summaries.

## Hegel RS properties

Generated manifests should verify stable sorting, no duplicate requirement sections, accurate gap groups, and consistency between summary counts and entry refs.

## Non-goals

- No log scraping as proof.
- No authority grant from readback artifacts.
