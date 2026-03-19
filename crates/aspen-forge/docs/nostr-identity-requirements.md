# Nostr Identity Requirements

## Keypair Mapping

r[identity.mapping.create]
The system MUST generate a unique ed25519 keypair for each Nostr npub on first authentication.

r[identity.mapping.retrieve]
The system MUST return the same ed25519 keypair for subsequent authentications with the same npub.

r[identity.mapping.isolation]
Different npubs MUST receive different ed25519 keypairs.

r[identity.mapping.encryption]
The stored ed25519 secret key MUST be encrypted at rest. The raw KV value MUST NOT contain the plaintext secret key.

r[identity.mapping.cluster-bound]
The encryption key MUST be derived from the cluster's iroh secret key. A different cluster key MUST fail to decrypt.

## Challenge-Response Authentication

r[identity.auth.challenge-unique]
Each challenge MUST contain 32 random bytes. Challenges MUST NOT be reusable after verification.

r[identity.auth.verify-signature]
The system MUST verify the secp256k1 Schnorr signature (BIP-340) over SHA-256 of the challenge bytes against the claimed npub.

r[identity.auth.reject-invalid]
An invalid signature MUST be rejected.

r[identity.auth.reject-unknown]
An unknown or expired challenge ID MUST be rejected.

r[identity.auth.stable-key]
Repeated authentications with the same npub MUST resolve to the same ed25519 keypair.

r[identity.auth.token-npub]
The issued capability token MUST carry the authenticated npub as a fact.

## Per-User Signing

r[identity.signing.distinct-authors]
Commits created by different npub users on the same repo MUST have distinct ed25519 author keys.

r[identity.signing.npub-in-author]
Commits created by an authenticated user MUST carry the user's npub in the Author.npub field.

r[identity.signing.backward-compat]
Commits created without authentication MUST have Author.npub = None and MUST continue to work.

## Encryption Invariants

r[identity.crypto.key-derivation]
The encryption key MUST be derived via BLAKE3 keyed hash with a fixed context string from the cluster secret key.

r[identity.crypto.nonce-unique]
Each encryption operation MUST use a fresh 24-byte random nonce.

r[identity.crypto.roundtrip]
encrypt(plaintext) followed by decrypt(ciphertext) MUST return the original plaintext when using the same key.

r[identity.crypto.wrong-key-fails]
decrypt with a different key MUST fail.
