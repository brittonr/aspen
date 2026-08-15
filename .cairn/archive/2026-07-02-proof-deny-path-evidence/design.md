# Design: deny-path evidence matrix

## Scope

This change makes negative evidence requirements explicit for proof-bearing gates. It does not require every operator diagnostic to be release-blocking; it applies to gates that can be used as pass evidence.

## Matrix model

Each proof-bearing gate should publish a deny-path matrix listing required negative classes, fixture refs or generated proof refs, expected decision, and mutation boundary evidence. Initial classes are missing-artifact, stale-ref, malformed-schema, wrong-signer, wrong-purpose, tampered-bytes, duplicate-receipt, denied-mutation, and diagnostic-only-not-pass.

## No-mutation proof

For gates that deny before side effects, the denial receipt should bind either unchanged state refs or an explicit no-mutation receipt. Logs are diagnostic-only.

## Hegel RS properties

Generated cases should vary schema tags, refs, signer/purpose fields, duplicate ids, and mutation intents. The property law is that no generated deny-class input can produce a pass receipt or mutate committed state.

## Non-goals

- No ambient security claim beyond the declared gate scope.
- No acceptance of logs as authoritative denial evidence.
