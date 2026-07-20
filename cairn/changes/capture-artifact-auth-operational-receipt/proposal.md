# Change: Capture Molten artifact-auth operational receipts

## Why

Molten's capability-file adapter already owns key generation, rotation, and durable revocation markers, but exact standalone shell evidence is transient. A persisted receipt must be replayed after reopening node state and rechecking the adapter's current key status.

## What Changes

- Build a deterministic BLAKE3-identified receipt from the exact standalone carrier and shell report.
- Persist it through the capability-rooted `Receipts` namespace.
- Reopen secret and receipt namespaces, derive currentness from the actual key record/revocation marker, and independently replay verification.
- Keep Molten verification/admission authoritative and standalone authority disabled.

## Scope

No transport, federation, membership, capability, entropy, rotation, policy, release, or authority semantics move into artifact-auth.
