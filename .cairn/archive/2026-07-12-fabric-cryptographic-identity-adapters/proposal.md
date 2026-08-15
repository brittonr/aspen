## Why

Molten has strong canonical identity, drift, rotation, and receipt models, but its current live endpoint and federation paths still use deterministic fixture-derived key material and local BLAKE3 fixture signatures. Those mechanisms are suitable for replay tests, not production transport or federation identity.

The fabric needs adapter-neutral cryptographic operations so production keys can be generated, stored, used, rotated, and revoked without exposing secret bytes to primitives, extensions, receipts, or operator readback.

## What Changes

- Keep identity, signature-domain, freshness, rotation, revocation, and admission decisions in pure deterministic primitives.
- Add versioned cryptographic signer, verifier, key-generation, and key-store adapter contracts using opaque key handles.
- Add an initial production Ed25519/Iroh identity profile backed by admitted cryptographic entropy and capability-rooted files or a managed secret backend.
- Separate transport, federation, evidence-signing, and authority key purposes unless reviewed policy explicitly permits a shared key.
- Reclassify deterministic BLAKE3 signature and endpoint-key derivation as fixture-only profiles that cannot satisfy production admission.
- Add positive and negative adapter conformance, restart, rotation, revocation, permission, redaction, and wrong-purpose tests.

## Impact

- **Files**: node identity, evidence and federation signature boundaries, fabric adapter descriptors, Iroh endpoint construction, secret persistence shells, operator readback, fixtures, and `cairn/specs/persistent-node-identity/spec.md`.
- **Testing**: production-profile key persistence and signing tests plus negative fixture-profile, unsafe-permission, wrong-key, wrong-purpose, stale-generation, revoked-key, malformed-signature, and secret-leak tests.
- **Safety**: public identities and successful signature verification remain evidence inputs only; they do not grant capabilities, membership, service authorization, provenance, or policy admission.
- **Licensing**: Aspen `main` is a behavior and test-design reference only; implementation code requires an explicit compatible source license or an independently written implementation.
