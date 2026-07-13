# Fabric cryptographic identity adapters

Molten's cryptographic identity boundary keeps key lifecycle and signature semantics separate from extensions, transport, and authority. Pure contracts live in `crates/molten-core/src/fabric_crypto_identity/`; capability-rooted persistence, Ed25519 effects, canonical Preserves evidence, and Iroh endpoint construction live in `src/fabric_crypto_identity/`.

## Production profile

The checked production profile is `docs/fabric-cryptographic-identity/profile.ncl`. It requires:

- Ed25519/Iroh keys generated from the operating-system CSPRNG;
- capability-rooted storage with owner-only permissions, or a separately reviewed managed backend;
- opaque, generation-scoped handles instead of private key bytes;
- separate transport, federation-origin, delegation, evidence-signing, and authority purposes;
- canonical payload references and the `molten.crypto.signature` versioned domain;
- generation-fenced rotation, explicit revocation evidence, and verification-only overlap;
- public-only bounded status with private bytes and backend locators redacted.

Node first boot writes a binary Ed25519 key record through the identity namespace at mode `0600`. Restart resolves the same public identity. Live Iroh endpoint construction reloads that admitted record through the capability root and checks the endpoint ID plus opaque handle before use. It no longer derives an endpoint secret from public node inputs.

## Signing and verification

`IrohEd25519FileAdapter` signs canonical domain records, not arbitrary ambient bytes. The signed record binds the profile, purpose, domain ID/version, payload schema/ref, signer public-key ref, and verifier-context ref. The shell returns canonical public signatures and outcomes. Federation and evidence admission helpers consume those outcomes without receiving adapter runtime objects or private material.

A transport key cannot sign federation, delegation, evidence, or authority records. Verification fails closed for wrong purpose, wrong public key, malformed signatures, stale generations, revocation, and payload-reference changes.

## Rotation and readback

Rotation is planned by the pure core before the shell writes a replacement key. The file adapter requires the current generation, current handle, current public key, backend, and profile to match. It persists the next generation with restricted permissions and emits public transition evidence. Old keys are revoked immediately for a no-overlap plan; overlap is verification-only and bounded by explicit policy.

`redacted_status` reports only profile, purpose, generation, backend class, permission state, currentness, public-key ref, opaque handle ref, and bounded receipt refs. It marks the backend locator as redacted and never serializes private bytes.

## Fixture boundary

The existing `blake3-local-fixture-v1` evidence and federation signatures remain compatibility fixtures. They are deterministic integrity fixtures, not production cryptographic identities. `blake3-local-fixture-v1`, deterministic entropy, and public-input-derived endpoint material are denied by the production profile and production adapter admission. They must not satisfy production startup, federation-origin, delegation, evidence-signing, or authority gates.

## Claim boundaries

Cryptographic verification establishes only that the supplied key verifies the canonical payload in the declared domain and currentness context. It does not establish membership, capability authority, trust-root selection, policy admission, provenance, release eligibility, or whole-system correctness. Private-key storage safety is bounded to the selected backend and recorded permission checks.
