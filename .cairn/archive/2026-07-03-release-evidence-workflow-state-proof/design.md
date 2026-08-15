# Design: release evidence workflow state proof

## Scope

This change proves release evidence workflow state-machine behavior. It covers dogfood evidence, bundle export, bundle verification, signed-member verification, keyring currentness, release promotion, signed promotion, summary/readback, evidence archive export, archive verification, and catalog discoverability.

## Proof checklist

- **Proof claim**: release review passes only when every required member, signature, purpose, key currentness, replay/proof ref, bundle manifest, and archive member binding is present and current; release evidence remains non-authorizing for subsystem gates.
- **Out of scope**: publishing, deployment, and external release-channel authority.
- **Trusted assumptions**: signed receipt verification and keyring records validate signatures and revocations correctly.
- **Positive evidence**: complete dogfood→bundle→verify→signed-member→promote→signed-promotion→summary→export→verify trace.
- **Negative evidence**: missing member, duplicate path, tampered ref, unsigned required member, wrong signer, wrong purpose, revoked key, stale replay/proof ref, and downstream evidence-only misuse deny.
- **Canonical refs**: dogfood report refs, bundle refs, signed member refs, keyring refs, promotion refs, signed promotion refs, summary refs, export manifest refs, archive member refs, and verification refs.
- **Regeneration command**: `cargo test dogfood receipts catalog`.

## Functional core

Represent release workflow checks as pure validation over member manifests, refs, signatures, key states, purposes, and required evidence classes. Shells perform archive IO and ledger import only after validation returns pass.

## Non-goals

- No release publication authority.
- No subsystem authority, provenance, source-gate, retention, or destructive-operation trust from release evidence.
