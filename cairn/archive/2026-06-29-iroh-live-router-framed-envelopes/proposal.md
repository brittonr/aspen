## Why

Molten already has local Iroh-shaped evidence, node-control live workflow receipts, and multi-node VM coverage, but the live transport boundary is still modeled as fixed helper flows rather than a runtime-managed protocol surface. The highest-value patterns from `n0-computer/iroh-examples` are `custom-router` and `framed-messages`: dynamic ALPN protocol registration/removal and bounded length-delimited bidirectional streams.

Molten should integrate those patterns in Molten terms: canonical Preserves envelopes, explicit admission gates, deterministic frame validation, bounded resource use, and receipt evidence that transport availability never grants authority or policy trust. `irpc` is also a useful reference for local/remote request, response, and streaming interaction patterns, but Molten should keep canonical Preserves frames rather than adopting postcard as an external wire contract.

## What Changes

- Add a runtime-managed Iroh protocol router boundary for node services that can install, replace, and remove ALPN handlers only through admitted control-plane operations.
- Add a bounded framed-envelope stream for direct Iroh bidirectional connections, carrying canonical Preserves envelope frames with declared refs and max-frame limits.
- Emit canonical receipts for protocol registration, replacement, removal, stream open/close, frame admission, frame denial, handler shutdown, and unsupported-ALPN denial.
- Bind router and framed-stream evidence into node-control live workflow and multi-node VM checks without replacing existing gossip/local-loopback evidence.
- Add positive and negative tests for accepted ALPNs, removed ALPNs, replacement generation checks, malformed frames, oversized frames, stale refs, unsupported protocols, and request/response or streaming service sessions over admitted framed streams.
- Treat the `iroh-examples` code as a design reference only; do not copy API compatibility claims or unbounded example defaults into Molten.

## Impact

This narrows the gap between Molten's current evidence-only Iroh adapter paths and a production-shaped live protocol boundary. It should improve node-control, plugin-host, protocol-session, and future peer-service transport work while preserving Molten's existing rule that live transport is evidence-only unless separately admitted by authority, policy, resource, provenance, and replay gates.
