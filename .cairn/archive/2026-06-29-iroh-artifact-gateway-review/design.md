## Overview

Use `n0-computer/iroh-examples/iroh-gateway` as a reference for read-only content serving patterns: byte-range parsing, chunk-range mapping, collection-style indexes, MIME hints, and an HTTP shell. Molten must wrap those ideas with its own content-addressed chunk store, catalog, retention, confidentiality, and evidence rails.

## Functional core

Add pure core types and functions for operator gateway decisions:

- `GatewayReadRequest`: requested object ref, optional path/member, byte range, requester/viewer context, visibility policy refs, and evidence refs.
- `GatewayReadDecision`: pass/deny/degraded, normalized byte range, object/ref classifications, required chunk refs, diagnostics, and checks.
- `GatewayIndexDecision`: pass/deny result for collection/index rendering with hidden/ref-redacted members removed.
- `GatewayMimeDecision`: optional MIME hint derived from policy-admitted metadata or bounded sniffed bytes.

Core validators must answer:

- Is the requested object ref canonical and visible?
- Is the requested range normalized and bounded?
- Which chunk refs are required for the response?
- Are retention, confidentiality, redaction, and reveal gates satisfied?
- Does a collection/index omit hidden or denied members?

## Imperative shell

The shell owns HTTP binding, Iroh connection setup if live blob fetch is used, request parsing, response streaming, and diagnostic logs. It must call the pure gateway decision core and chunk-store verification before writing response bytes.

A first implementation can be a CLI fixture that emits gateway receipts and verified byte ranges without starting a long-running HTTP server. The HTTP service can follow once the readback semantics are stable.

## Range and chunk verification

Range requests must map byte ranges to Molten chunk-manifest ranges and verify every relevant chunk before response. Missing, corrupt, wrong-length, reordered, or unsupported-transform chunks deny before bytes are exposed. Partial response receipts must bind the original manifest ref, normalized range, chunk refs, and verification result.

## Visibility and redaction

The gateway must treat names, MIME hints, sizes, collection members, and short refs as potentially sensitive. Public or diagnostic profiles may return redacted index entries, omit sensitive members, or require explicit reveal receipts. Plaintext bytes for protected commitments must deny unless the caller supplies reveal authority and evidence accepted by existing confidentiality/retention gates.

## Receipts

Add canonical evidence such as:

- `operator-gateway-read-receipt-v1`,
- `operator-gateway-index-receipt-v1`,
- `operator-gateway-range-receipt-v1`.

These receipts are readback evidence only. They do not grant authority, policy admission, provenance trust, source-gate acceptance, retention clearance, or execution rights.

## Tests

Positive tests should cover full-object readback, bounded range readback, collection/index rendering with visible members, and MIME hint binding.

Negative tests should cover malformed refs, unsatisfied retention/reveal gates, hidden members, unsupported transforms, corrupt chunks, range overflows, multiple or invalid ranges, and attempts to use gateway receipts as mutation authority.

## Non-goals

- No public unauthenticated gateway in the first slice.
- No claim of compatibility with the `iroh-gateway` URL surface.
- No write, delete, mutate, pin, unpin, or GC operations.
- No replacement for catalog/MCP authorization or retention gates.
