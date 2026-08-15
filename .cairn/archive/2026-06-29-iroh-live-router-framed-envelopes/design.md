## Overview

Use `n0-computer/iroh-examples` and `n0-computer/irpc` as references for three implementation patterns:

- `custom-router`: a mutable ALPN router that can add, replace, remove, and gracefully shut down protocol handlers.
- `framed-messages`: length-delimited framing over Iroh bidirectional streams.
- `irpc`: local/remote request, response, server-streaming, client-streaming, and bidirectional-streaming service interactions.

Molten must not import those examples as product semantics. The Molten design keeps the functional core pure and deterministic, while the imperative shell owns Iroh endpoint binding, accept loops, stream I/O, cancellation, timeouts, and filesystem/CLI interaction. External Molten wire identity remains canonical Preserves, even where IRPC's Rust-only/postcard design inspires the shape of service sessions.

## Functional core

Add pure core types and validators for the live protocol boundary:

- `IrohAlpnId`: bounded byte/string identifier with canonical display and allowed prefix policy.
- `IrohProtocolRegistration`: requested ALPN, handler kind, generation, authority refs, policy refs, resource refs, and evidence refs.
- `IrohProtocolRegistry`: deterministic map from ALPN to handler descriptor and generation.
- `IrohProtocolDecision`: pass/deny result for install, replace, remove, and shutdown operations.
- `FramedEnvelopeLimits`: max frame bytes, max frames per session, max outstanding frames, and close-drain policy.
- `FramedEnvelopeInput`: declared envelope ref, canonical Preserves bytes, direction, ALPN, peer, sequence, and evidence refs.
- `FramedEnvelopeDecision`: pass/deny result with parsed envelope ref, checks, diagnostics, and replay classification.

The core must answer questions such as:

- Is this ALPN syntactically valid and policy-admitted?
- Does replacement advance the generation and preserve shutdown evidence for the previous handler?
- Does a frame stay within limits?
- Do the bytes parse as canonical Preserves and hash to the declared envelope ref?
- Does an unsupported ALPN, stale frame ref, malformed frame, or oversized frame deny before delivery?

## Imperative shell

The shell owns actual Iroh operations:

- bind an endpoint with the current ALPN set,
- accept incoming connections,
- dispatch accepted connections to a handler registered in the admitted registry,
- open bidirectional streams for direct node-control or protocol-session traffic,
- encode/decode length-delimited frames,
- apply cancellation and bounded shutdown,
- write receipts and diagnostic logs.

The shell must be intentionally boring: it calls the pure validators before mutating the live router or delivering a frame, then records the resulting receipt.

## Router semantics

The router has generationed registrations so replacement is explicit and auditable. Installing a new ALPN emits a pass receipt with `outcome=inserted`; replacing emits `outcome=replaced` and includes the previous handler generation plus shutdown evidence. Removing an ALPN emits a pass receipt only after the registry no longer advertises it. Unknown removal emits deny evidence.

Existing accepted connections may finish according to the handler's drain policy, but new connections must observe the latest registry. Unsupported ALPN connections must deny or close without delivering frames or mutating node state.

## Framed envelope stream

Frames use a deterministic length-delimited envelope format over Iroh bidirectional streams. The frame payload is canonical Preserves bytes, not Rust serialization or debug text. Each frame admission receipt binds:

- ALPN,
- peer/node ids,
- stream/session id,
- sequence,
- declared envelope ref,
- actual canonical hash,
- frame length,
- limit profile ref,
- pass/deny checks.

The first implementation may use a fixed local frame codec and test harness rather than expose a stable public wire contract. If a public wire contract is introduced, it must be versioned and documented separately.

## Streaming service sessions

IRPC's useful idea is not its exact serialization; it is the typed interaction model: unary request/response, server streaming, client streaming, and bidirectional streaming over cheap QUIC/Iroh streams. Molten can model those as admitted service-session descriptors whose frames are canonical Preserves envelopes and whose channels are explicit capability-bearing endpoints.

A service session decision should bind the service id, method or operation id, interaction kind, stream direction, ALPN, peer/node ids, capability refs, policy refs, resource refs, and replay classification. Streaming updates are ordinary framed envelope decisions with sequence and flow-control evidence. Local in-process service calls may use a zero-copy shell, but their admitted request/response records must be the same canonical model used for remote sessions.

## Receipts

Add canonical receipt families such as:

- `iroh-protocol-router-receipt-v1`,
- `iroh-framed-envelope-receipt-v1`,
- `iroh-stream-session-receipt-v1`.

Receipt decisions remain evidence-only. A passing router or stream receipt does not grant authority, policy, resource, provenance, source-gate, retention, or deterministic replay trust by itself.

## Tests and validation

Positive tests should cover installing, replacing, removing, and using admitted ALPN handlers, plus successful frame delivery of canonical Preserves envelopes.

Negative tests should cover unsupported ALPN, duplicate stale generation, unknown removal, malformed frame bytes, oversized frames, mismatched declared refs, missing policy/authority evidence, shutdown timeout, and live unrecorded delivery being excluded from deterministic pass evidence.

The NixOS multi-node VM check should bind child refs for at least one framed direct stream path once the shell exists. If VM support is unavailable, the check must not mint pass evidence.

## Non-goals

- No compatibility claim with `iroh-examples` APIs or examples.
- No replacement of existing gossip or local-loopback receipts in the first slice.
- No browser/Wasm integration.
- No transport-derived authority.
