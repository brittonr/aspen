# Design: Native callback value materialization

## Context

The accepted native host uses exact BLAKE3 references, but its process callback sees no application bytes. The callback outcome also returns references without publishable bodies. This prevents an external executable from owning semantic state transitions.

The change must preserve the functional core and imperative shell. Pure framing and admission own identity and bounds. The shell owns value reads, publications, process execution, journaling, and provider effects.

## Decisions

### 1. Publish protocol v2 without reference-only fallback

The v2 envelope embeds optional payload and prior-state values as exact `(reference, bytes)` pairs. The v2 outcome embeds output, effect-request, next-state, and checkpoint values in the same form. Decoders verify BLAKE3 identity, canonical framing, item limits, individual value limits, and total frame limits.

A v2 profile accepts only the v2 schema, ALPN, and framing. Missing bytes are a denial, not a request to use v1.

### 2. Add a narrow value port

`NativeCallbackValuePort` provides two external capabilities:

- materialize bounded bytes for an exact content reference;
- publish bounded bytes under their exact content reference.

The port does not assign application meaning. It does not authorize callbacks or provider effects. An in-memory adapter supplies deterministic tests; deployment adapters remain composition-owned.

### 3. Commit intent before every external effect

The host derives callback operation identity from stable invocation facts. It persists callback intent before value materialization or process execution.

For every returned body, it persists a publication operation before calling the value port. A definite rejection becomes terminal. An uncertain publication remains unknown and blocks automatic retry. Provider routing still requires its existing durable effect intent.

### 4. Separate semantic state from lifecycle checkpoints

The durable instance stores `state_ref` and `checkpoint_ref` independently. Request callbacks receive the latest state bytes. Checkpoint callbacks can publish an explicit checkpoint body. Recovery materializes the exact checkpoint and state selected by the host.

### 5. Keep core outcome types reference-based

The process wire carries bytes. The executor verifies and publishes those bytes, then projects the result into the existing reference-based `CallbackOutcome`. The generic system-extension core remains independent of storage adapters.

## Failure semantics

- Missing, corrupt, substituted, or oversized input fails before process start.
- Malformed or reference-only output fails before publication or provider routing.
- Publication rejection is terminal and leaves semantic state unchanged.
- Publication uncertainty remains `Unknown`; no implicit retry occurs.
- A crash after intent but before observed completion is visible in recovery inventory.
- Provider effects cannot run until all required request bodies are durably published.

## Validation

Positive tests cover ingress, state, output, effect body, checkpoint, restart, and recovery. Negative tests cover missing bytes, wrong identity, bounds, legacy framing, malformed output, publication rejection, publication uncertainty, stale generation, and restart inventory.
