# Design: exact artifact-auth shell verification pilot

## Completion contract

The pilot is complete only when a production-managed Molten Ed25519 key signs the canonical bytes returned by `artifact_auth_core::canonical_statement_bytes`, `artifact-auth-ed25519` verifies that exact reconstructed statement, and deterministic tests show valid parity plus fail-closed tamper, wrong-preimage, wrong-key, malformed-signature, currentness, and carrier-integrity behavior. A supplied boolean, a legacy domain signature, a fixture-only key, test count, or `standalone_authority_admitted = true` is false completion.

## Functional core and imperative shell

`molten-core` owns a pure statement-input type and deterministic statement mapping. The root Molten crate owns public-key parsing, capability-file key loading, signing, signature verification, public evidence rendering, and orchestration. The shell never receives raw private bytes and the core performs no filesystem, process, network, clock, JSON, or signing effects.

## Exact preimage

The shell asks the core for the signer-specific `ArtifactStatement`, canonicalizes it with the pinned standalone core, and signs only those bytes. Verification reconstructs the statement independently from supplied product observations and verifies that statement directly with `artifact-auth-ed25519`. Legacy `CanonicalSignatureDomain` bytes and `cryptographic_verification_passed` are distinct inputs and cannot satisfy this verification.

## Carrier integrity and identities

The public carrier records the BLAKE3 statement ref, public-key ref, signature ref, signature bytes, and lowercase signature hex. Verification recomputes every identity before cryptographic evaluation. Full public-key identity is derived from raw key bytes; labels never substitute for key identity.

## Authority and rollback

The resulting cryptographic observation enters the existing pure dual-run comparator. The comparator continues to emit `legacy_authoritative = true`, `standalone_authority_admitted = false`, and `rollback_available = true`. Rollback stops calling the shell pilot and continues legacy verification; key storage, signing permission, currentness, membership, capability, transport, deployment, and release policy remain Molten-owned.

## Approach registry

| Family | Mechanism | State | Reason |
| --- | --- | --- | --- |
| Reuse legacy canonical-domain signatures | Feed the existing domain signature or legacy boolean into standalone evaluation | Falsified | It covers a different preimage. |
| Export a generic private-key signer | Let callers obtain key material or unrestricted signing | Rejected | It widens key and signing authority. |
| Fixture-only standalone signing | Verify a deterministic standalone test key without the production adapter | Blocked | It does not establish product-shell integration. |
| Purpose-bounded exact-statement shell | Keep private keys in the adapter, map in pure core, sign exact standalone bytes, verify with the pinned verifier | Selected | It supplies discriminating evidence without authority admission. |

## Audit risks

The adversarial pass targets statement reconstruction drift, purpose/profile mismatch, public-key encoding and full-key identity, signature-carrier substitution, wrong preimages, malformed lengths, revoked or superseded currentness, unrelated-failure parity, secret leakage, and accidental authority promotion.
