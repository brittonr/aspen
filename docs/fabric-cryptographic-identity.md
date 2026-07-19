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

## Standalone artifact-auth compatibility

Molten pins `artifact-auth-core` from `ssh://git@github.com/OnixResearch/artifact-auth.git` at revision `799459346d5416fbd7b9f55840a7371441b55afa`. Cargo and the non-flake Nix input must resolve that same full revision; flake evaluation rejects duplicate lock packages, source mismatch, or a standalone license other than `MIT OR Apache-2.0`. SSH credentials authorize retrieval only.

`fabric_crypto_identity::evaluate_artifact_auth_dual_run` maps an already measured Molten verification request plus a separate `CryptographicObservation` for the exact standalone statement. The legacy `cryptographic_verification_passed` field is never reused as proof of the standalone preimage. The adapter preserves domain, purpose, profile, payload, public-key, verifier-context, generation, and currentness fields while keeping opaque handles, backend class, entropy profile, and rotation transitions as Molten-owned extensions.

Compatibility output classifies the intentionally distinct canonical preimages, full-key identity, decisions, consumer-specific issue taxonomy, mandatory non-claims, and unrelated-failure false parity. `legacy_authoritative` and `rollback_available` remain true; `standalone_authority_admitted` remains false. This adoption therefore rejects runtime authority cutover until a later change supplies shell integration and current operational evidence.

### Update and rollback

To update, review the candidate standalone release and `config/consumers/molten.ncl`, change the exact Cargo and Nix revisions together, regenerate `Cargo.lock` with Cargo and `flake.lock` with Nix, then rerun focused identity tests, strict Clippy/Octet, Cairn validation, and the Nix checks. Never edit either lock manually.

Runtime rollback is immediate: stop supplying a standalone cryptographic observation and continue evaluating the legacy Molten decision. Dependency rollback restores the last reviewed Cargo/Nix declarations as one VCS change, regenerates both locks with their owning tools, and preserves the compatibility evidence explaining the rejection.

## Claim boundaries

Cryptographic verification establishes only that the supplied key verifies the canonical payload in the declared domain and currentness context. It does not establish membership, capability authority, trust-root selection, policy admission, provenance, release eligibility, or whole-system correctness. Private-key storage safety is bounded to the selected backend and recorded permission checks. Standalone success additionally does not authorize signing, key generation, storage, rotation, capability/federation decisions, Preserves/Iroh transport, runtime admission, deployment, or release.
