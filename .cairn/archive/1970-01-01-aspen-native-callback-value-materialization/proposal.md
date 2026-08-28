# Change: Materialize native callback values

## Why

The native callback protocol carries content references without the referenced bytes. An extension cannot inspect ingress, prior state, effect completions, or checkpoints from a BLAKE3 reference alone. It also cannot return new state or effect request bodies for verified publication.

This gap blocks real hosted consumers. A local model can simulate the missing values, but that model is not native-host evidence.

## What Changes

- Publish a new exact native callback protocol version with bounded reference-and-byte values.
- Add an application-owned value port for identity-checked materialization and publication.
- Persist callback and publication intent before any materialization, process, publication, or provider effect.
- Track the latest semantic state reference separately from lifecycle checkpoints.
- Materialize ingress, state, effect completion, next-state, checkpoint, output, and effect-request bodies.
- Fail closed on missing, corrupt, oversized, uncertain, or reference-only values.
- Add positive and negative separate-process, restart, and recovery evidence.

## Impact

- Affected specification: `system-extension-runtime`
- Affected code: native host model, wire format, executor, service, journal, fixtures, profiles, and tests
- Compatibility: protocol v1 remains historical evidence only; the new host profile does not fall back to it
- Non-claims: byte identity does not prove callback correctness, provider success, sandboxing, or release readiness
