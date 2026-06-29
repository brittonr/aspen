## Why

Molten already stores artifacts, receipts, chunks, retention bundles, and release evidence as content-addressed Preserves values. Operators need safe readback and review workflows, especially for large chunked artifacts and evidence bundles. The `iroh-gateway` example shows useful patterns for a stateless HTTP gateway over Iroh blobs, including range requests, collection indexes, and MIME sniffing.

Molten should consider a read-only operator gateway, but only in Molten terms: canonical refs, receipt-backed access decisions, retention/redaction policy, range-read verification, and explicit evidence-only semantics. This should be a follow-up after the core live router/framed-envelope work, not a prerequisite for node-control correctness.

## What Changes

- Define a read-only operator gateway for content-addressed Molten artifacts, chunks, receipts, and evidence bundles.
- Adapt the useful `iroh-gateway` reference patterns: byte-range mapping, collection/index views, cache bounds, and optional HTTP service shell.
- Require every served byte range to pass chunk-store manifest and chunk verification before response.
- Require retention, confidentiality, redaction, and visibility checks before exposing artifact names, refs, MIME hints, or bytes.
- Emit gateway readback receipts that bind request, decision, object refs, range, visibility policy, and diagnostics.
- Keep the gateway out of authority, policy, source-gate, provenance, and destructive-operation trust paths.

## Impact

This improves operator UX for large evidence and artifact review without changing runtime semantics. It is intentionally read-only and should fail closed whenever policy, retention, confidentiality, or chunk verification is incomplete. It can share concepts with `iroh-gateway`, but must not copy its unauthenticated stateless behavior as Molten product behavior.
