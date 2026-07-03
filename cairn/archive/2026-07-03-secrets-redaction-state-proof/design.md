# Design: secrets redaction state proof

## Scope

This change proves secret, encrypted-ref, reveal/decrypt, redaction, repro-profile, and cleanup state-machine behavior. It covers default redaction, reveal authority, decrypt receipts, gate-preserving transform claims, diagnostic redaction, private encrypted bundles, reveal receipt binding, and retention-gated cleanup.

## Proof checklist

- **Proof claim**: plaintext is exposed only through passing reveal/decrypt authority bound to the exact secret or encrypted-ref ids; public/diagnostic profiles cannot satisfy pass gates unless a gate-preserving transform is explicitly proven.
- **Out of scope**: encryption primitive security proofs and key-management UX beyond receipt/key refs.
- **Trusted assumptions**: encrypted-ref commitments and canonical refs are collision-resistant under BLAKE3.
- **Positive evidence**: authorized reveal/decrypt traces bind secret refs, encrypted-ref ids, commitments, policy, authority, resource, effect, and output refs.
- **Negative evidence**: ciphertext-only reveal, missing authority, stale reveal, wrong encrypted-ref id, commitment mismatch, diagnostic redaction used as pass evidence, and cleanup without retention admission deny.
- **Canonical refs**: secret ref, encrypted-ref ref/id, reveal/decrypt receipt refs, redaction transform refs, profile refs, cleanup refs, retention refs, and output refs.
- **Regeneration command**: `cargo test secrets repro gate`.

## Functional core

Expose pure decisions for reveal, decrypt, redaction profile admissibility, gate-preserving transforms, and cleanup admission. Rendering and filesystem export remain imperative shells that never see plaintext unless the pure decision passes.

## Non-goals

- No plaintext-by-default rendering.
- No gate-preserving claim for diagnostic redaction profiles.
