//! Self-contained credential type for cross-cluster authorization.
//!
//! A `Credential` bundles a capability token with its full delegation proof chain,
//! enabling offline verification without server-side state or network calls.
//! This implements the UCAN invocation model with Aspen's native encoding
//! (postcard + BLAKE3, not JWT + CID).

use std::time::Duration;

use iroh_base::PublicKey;
use iroh_base::SecretKey;
use serde::Deserialize;
use serde::Serialize;

use crate::builder::TokenBuilder;
use aspen_auth_core::Capability;
use aspen_auth_core::CapabilityToken;
use aspen_auth_core::constants::MAX_DELEGATION_DEPTH;
use aspen_auth_core::constants::MAX_TOKEN_SIZE;

use crate::AuthError;
use crate::verifier::TokenVerifier;

/// Maximum credential size: bounded by delegation depth × token size.
pub const MAX_CREDENTIAL_SIZE: usize = MAX_DELEGATION_DEPTH as usize * MAX_TOKEN_SIZE as usize;

/// A self-contained credential for cross-cluster authorization.
///
/// Bundles a leaf capability token with its full delegation proof chain
/// (ordered leaf-parent to root). Verifiable offline using only crypto
/// operations on the credential contents.
///
/// # Wire encoding
///
/// Uses postcard for compact binary serialization. Size is bounded by
/// `MAX_DELEGATION_DEPTH × MAX_TOKEN_SIZE` (~64KB worst case, typically ~3KB).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    /// The leaf capability token being presented.
    pub token: CapabilityToken,
    /// Delegation proof chain, ordered from immediate parent to root.
    /// Empty for root tokens (depth 0).
    pub proofs: Vec<CapabilityToken>,
}

impl Credential {
    /// Create a root credential (depth 0, no delegation chain).
    pub fn from_root(token: CapabilityToken) -> Self {
        Self {
            token,
            proofs: Vec::new(),
        }
    }

    /// Verify the credential's token and delegation chain.
    ///
    /// Walks the chain from leaf to root, verifying signatures, expiry,
    /// and capability attenuation at each level. No server-side state or
    /// network calls are needed.
    ///
    /// # Arguments
    ///
    /// * `trusted_roots` - Public keys of trusted root issuers
    /// * `presenter` - Optional public key of who is presenting the credential
    pub fn verify(&self, trusted_roots: &[PublicKey], presenter: Option<&PublicKey>) -> Result<(), AuthError> {
        let mut verifier = TokenVerifier::new();
        for root in trusted_roots {
            verifier = verifier.with_trusted_root(*root);
        }
        verifier.verify_with_chain(&self.token, &self.proofs, presenter)
    }

    /// Delegate this credential to create a child credential.
    ///
    /// The new credential has narrower capabilities and a new audience.
    /// The proof chain grows by one level (the current token is appended).
    ///
    /// # Arguments
    ///
    /// * `issuer_key` - Secret key of the current credential holder (must match token audience)
    /// * `audience` - Public key of the new delegate
    /// * `capabilities` - Capabilities to grant (must be subset of current)
    /// * `lifetime` - How long the delegated token should be valid
    ///
    /// # Errors
    ///
    /// Returns error if delegation depth would exceed `MAX_DELEGATION_DEPTH`,
    /// capabilities are not a subset, or the current token doesn't allow delegation.
    pub fn delegate(
        &self,
        issuer_key: &SecretKey,
        audience: PublicKey,
        capabilities: Vec<Capability>,
        lifetime: Duration,
    ) -> Result<Self, AuthError> {
        // Build the child token delegated from the current leaf
        let child_token = TokenBuilder::new(issuer_key.clone())
            .for_key(audience)
            .with_capabilities(capabilities)
            .with_lifetime(lifetime)
            .with_random_nonce()
            .delegated_from(self.token.clone())
            .build()?;

        // Build the new proof chain: current leaf + existing proofs
        let mut new_proofs = Vec::with_capacity(self.proofs.len().saturating_add(1));
        new_proofs.push(self.token.clone());
        new_proofs.extend(self.proofs.iter().cloned());

        Ok(Self {
            token: child_token,
            proofs: new_proofs,
        })
    }

    /// Encode credential to bytes for wire transmission.
    pub fn encode(&self) -> Result<Vec<u8>, AuthError> {
        let bytes = postcard::to_allocvec(self).map_err(|e| AuthError::EncodingError(e.to_string()))?;
        if bytes.len() > MAX_CREDENTIAL_SIZE {
            return Err(AuthError::TokenTooLarge {
                size_bytes: bytes.len() as u64,
                max_bytes: MAX_CREDENTIAL_SIZE as u64,
            });
        }
        Ok(bytes)
    }

    /// Decode credential from bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, AuthError> {
        if bytes.len() > MAX_CREDENTIAL_SIZE {
            return Err(AuthError::TokenTooLarge {
                size_bytes: bytes.len() as u64,
                max_bytes: MAX_CREDENTIAL_SIZE as u64,
            });
        }
        Ok(postcard::from_bytes(bytes)?)
    }

    /// Encode to base64 for text transmission.
    pub fn to_base64(&self) -> Result<String, AuthError> {
        use base64::Engine;
        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(self.encode()?))
    }

    /// Decode from base64.
    pub fn from_base64(s: &str) -> Result<Self, AuthError> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s)?;
        Self::decode(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_secret_key() -> SecretKey {
        SecretKey::generate(&mut rand::rng())
    }

    #[test]
    fn test_credential_encode_decode_roundtrip() {
        let sk = test_secret_key();
        let token = TokenBuilder::new(sk)
            .with_capability(Capability::Read { prefix: "test:".into() })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();

        let cred = Credential::from_root(token);
        let bytes = cred.encode().unwrap();
        let decoded = Credential::decode(&bytes).unwrap();

        assert_eq!(decoded.token.issuer, cred.token.issuer);
        assert!(decoded.proofs.is_empty());
    }

    #[test]
    fn test_credential_base64_roundtrip() {
        let sk = test_secret_key();
        let token = TokenBuilder::new(sk)
            .with_capability(Capability::Read { prefix: "test:".into() })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();

        let cred = Credential::from_root(token);
        let b64 = cred.to_base64().unwrap();
        let decoded = Credential::from_base64(&b64).unwrap();

        assert_eq!(decoded.token.issuer, cred.token.issuer);
    }

    #[test]
    fn test_credential_with_delegation_chain_roundtrip() {
        let root_sk = test_secret_key();
        let child_sk = test_secret_key();
        let child_pk = child_sk.public();

        let root_token = TokenBuilder::new(root_sk)
            .for_key(child_pk)
            .with_capability(Capability::Read { prefix: "data:".into() })
            .with_capability(Capability::Delegate)
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();

        let grandchild_pk = test_secret_key().public();
        let root_cred = Credential::from_root(root_token);
        let child_cred = root_cred
            .delegate(
                &child_sk,
                grandchild_pk,
                vec![Capability::Read {
                    prefix: "data:sub:".into(),
                }],
                Duration::from_secs(1800),
            )
            .unwrap();

        assert_eq!(child_cred.proofs.len(), 1);

        let bytes = child_cred.encode().unwrap();
        let decoded = Credential::decode(&bytes).unwrap();

        assert_eq!(decoded.proofs.len(), 1);
        assert_eq!(decoded.token.delegation_depth, 1);
    }

    #[test]
    fn test_credential_size_bounded() {
        let sk = test_secret_key();
        let token = TokenBuilder::new(sk)
            .with_capability(Capability::Read { prefix: "test:".into() })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();

        let cred = Credential::from_root(token);
        let bytes = cred.encode().unwrap();
        assert!(bytes.len() < MAX_CREDENTIAL_SIZE);
    }

    #[test]
    fn test_root_credential_verification() {
        let root_sk = test_secret_key();
        let root_pk = root_sk.public();

        let token = TokenBuilder::new(root_sk)
            .with_capability(Capability::Read { prefix: "data:".into() })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();

        let cred = Credential::from_root(token);
        // Verify with the issuer as trusted root, no presenter (bearer)
        assert!(cred.verify(&[root_pk], None).is_ok());
    }

    #[test]
    fn test_two_level_chain_verification() {
        let root_sk = test_secret_key();
        let root_pk = root_sk.public();
        let child_sk = test_secret_key();
        let child_pk = child_sk.public();
        let presenter_pk = test_secret_key().public();

        // Root issues to child
        let root_token = TokenBuilder::new(root_sk)
            .for_key(child_pk)
            .with_capability(Capability::Read { prefix: "data:".into() })
            .with_capability(Capability::Delegate)
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();

        let root_cred = Credential::from_root(root_token);

        // Child delegates to presenter
        let child_cred = root_cred
            .delegate(
                &child_sk,
                presenter_pk,
                vec![Capability::Read {
                    prefix: "data:sub:".into(),
                }],
                Duration::from_secs(1800),
            )
            .unwrap();

        assert_eq!(child_cred.proofs.len(), 1);
        assert!(child_cred.verify(&[root_pk], Some(&presenter_pk)).is_ok());
    }

    #[test]
    fn test_three_level_chain_verification() {
        let root_sk = test_secret_key();
        let root_pk = root_sk.public();
        let mid_sk = test_secret_key();
        let mid_pk = mid_sk.public();
        let leaf_sk = test_secret_key();
        let leaf_pk = leaf_sk.public();
        let presenter_pk = test_secret_key().public();

        // Root -> Mid
        let root_token = TokenBuilder::new(root_sk)
            .for_key(mid_pk)
            .with_capability(Capability::Full { prefix: "data:".into() })
            .with_capability(Capability::Delegate)
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();

        let root_cred = Credential::from_root(root_token);

        // Mid -> Leaf
        let mid_cred = root_cred
            .delegate(
                &mid_sk,
                leaf_pk,
                vec![
                    Capability::Read {
                        prefix: "data:sub:".into(),
                    },
                    Capability::Delegate,
                ],
                Duration::from_secs(1800),
            )
            .unwrap();

        // Leaf -> Presenter
        let leaf_cred = mid_cred
            .delegate(
                &leaf_sk,
                presenter_pk,
                vec![Capability::Read {
                    prefix: "data:sub:deep:".into(),
                }],
                Duration::from_secs(900),
            )
            .unwrap();

        assert_eq!(leaf_cred.proofs.len(), 2);
        assert_eq!(leaf_cred.token.delegation_depth, 2);
        assert!(leaf_cred.verify(&[root_pk], Some(&presenter_pk)).is_ok());
    }

    #[test]
    fn test_max_depth_chain() {
        // MAX_DELEGATION_DEPTH = 8. Root token has depth 0.
        // Delegation check: new_depth = parent.depth + 1, fail if > MAX_DELEGATION_DEPTH.
        // So max reachable depth is 8, meaning 8 delegations from root (depth 0).
        // We need keys[0..=MAX_DELEGATION_DEPTH+1] to have room for one extra attempt.
        let skeys: Vec<SecretKey> = (0..=(MAX_DELEGATION_DEPTH + 2)).map(|_| test_secret_key()).collect();

        let root_token = TokenBuilder::new(skeys[0].clone())
            .for_key(skeys[1].public())
            .with_capability(Capability::Read { prefix: "".into() })
            .with_capability(Capability::Delegate)
            .with_lifetime(Duration::from_secs(86400))
            .build()
            .unwrap();

        let mut cred = Credential::from_root(root_token);

        // Delegate MAX_DELEGATION_DEPTH times (depth 1..=MAX_DELEGATION_DEPTH)
        for i in 1..=MAX_DELEGATION_DEPTH {
            let audience = skeys[(i + 1) as usize].public();
            cred = cred
                .delegate(
                    &skeys[i as usize],
                    audience,
                    vec![Capability::Read { prefix: "".into() }, Capability::Delegate],
                    Duration::from_secs(3600),
                )
                .unwrap();
        }

        assert_eq!(cred.token.delegation_depth, MAX_DELEGATION_DEPTH);
        assert_eq!(cred.proofs.len(), MAX_DELEGATION_DEPTH as usize);

        // Verify the full chain
        let presenter_pk = skeys[(MAX_DELEGATION_DEPTH + 1) as usize].public();
        assert!(cred.verify(&[skeys[0].public()], Some(&presenter_pk)).is_ok());

        // One more delegation should fail (exceeds max depth)
        let extra_pk = skeys[(MAX_DELEGATION_DEPTH + 2) as usize].public();
        let result = cred.delegate(
            &skeys[(MAX_DELEGATION_DEPTH + 1) as usize],
            extra_pk,
            vec![Capability::Read { prefix: "".into() }],
            Duration::from_secs(600),
        );
        assert!(matches!(result, Err(AuthError::DelegationTooDeep { .. })));
    }

    #[test]
    fn test_broken_chain_rejected() {
        let root_sk = test_secret_key();
        let root_pk = root_sk.public();
        let child_sk = test_secret_key();
        let child_pk = child_sk.public();

        // Create a root token for a different audience
        let root_token = TokenBuilder::new(root_sk)
            .for_key(child_pk)
            .with_capability(Capability::Read { prefix: "data:".into() })
            .with_capability(Capability::Delegate)
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();

        // Create a different, unrelated root token
        let unrelated_sk = test_secret_key();
        let unrelated_token = TokenBuilder::new(unrelated_sk)
            .with_capability(Capability::Read {
                prefix: "other:".into(),
            })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();

        // Build child delegated from root_token
        let child_token = TokenBuilder::new(child_sk.clone())
            .with_capability(Capability::Read {
                prefix: "data:sub:".into(),
            })
            .with_lifetime(Duration::from_secs(1800))
            .delegated_from(root_token)
            .build()
            .unwrap();

        // Put the unrelated token as the proof instead of the real parent
        let broken_cred = Credential {
            token: child_token,
            proofs: vec![unrelated_token],
        };

        // Verification should fail — proof hash doesn't match
        let result = broken_cred.verify(&[root_pk], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_capability_escalation_rejected() {
        let root_sk = test_secret_key();
        let child_sk = test_secret_key();
        let child_pk = child_sk.public();

        // Root grants only Read for "data:"
        let root_token = TokenBuilder::new(root_sk)
            .for_key(child_pk)
            .with_capability(Capability::Read { prefix: "data:".into() })
            .with_capability(Capability::Delegate)
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();

        let root_cred = Credential::from_root(root_token);

        // Child tries to delegate Write (escalation)
        let grandchild_pk = test_secret_key().public();
        let result = root_cred.delegate(
            &child_sk,
            grandchild_pk,
            vec![Capability::Write { prefix: "data:".into() }],
            Duration::from_secs(1800),
        );

        assert!(matches!(result, Err(AuthError::CapabilityEscalation { .. })));
    }

    #[test]
    fn test_delegation_outside_prefix_rejected() {
        let root_sk = test_secret_key();
        let child_sk = test_secret_key();
        let child_pk = child_sk.public();

        let root_token = TokenBuilder::new(root_sk)
            .for_key(child_pk)
            .with_capability(Capability::Read {
                prefix: "_sys:nix-cache:".into(),
            })
            .with_capability(Capability::Delegate)
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();

        let root_cred = Credential::from_root(root_token);

        // Child tries to delegate for a different prefix
        let grandchild_pk = test_secret_key().public();
        let result = root_cred.delegate(
            &child_sk,
            grandchild_pk,
            vec![Capability::Read {
                prefix: "_forge:repos:".into(),
            }],
            Duration::from_secs(1800),
        );

        assert!(matches!(result, Err(AuthError::CapabilityEscalation { .. })));
    }

    #[test]
    fn test_delegation_without_delegate_cap_rejected() {
        let root_sk = test_secret_key();
        let child_sk = test_secret_key();
        let child_pk = child_sk.public();

        // No Delegate capability
        let root_token = TokenBuilder::new(root_sk)
            .for_key(child_pk)
            .with_capability(Capability::Read { prefix: "data:".into() })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();

        let root_cred = Credential::from_root(root_token);

        let grandchild_pk = test_secret_key().public();
        let result = root_cred.delegate(
            &child_sk,
            grandchild_pk,
            vec![Capability::Read {
                prefix: "data:sub:".into(),
            }],
            Duration::from_secs(1800),
        );

        assert!(matches!(result, Err(AuthError::DelegationNotAllowed)));
    }

    #[test]
    fn test_token_with_facts_backward_compat() {
        let sk = test_secret_key();

        // Build a token WITH facts
        let token_with_facts = TokenBuilder::new(sk.clone())
            .with_capability(Capability::Read { prefix: "data:".into() })
            .with_fact("sync_interval", b"60".to_vec())
            .with_fact("region", b"us-east".to_vec())
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();

        assert_eq!(token_with_facts.facts.len(), 2);

        // Build a token WITHOUT facts
        let token_no_facts = TokenBuilder::new(sk)
            .with_capability(Capability::Read { prefix: "data:".into() })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();

        assert!(token_no_facts.facts.is_empty());

        // Both should verify correctly
        let verifier = TokenVerifier::new();
        assert!(verifier.verify(&token_with_facts, None).is_ok());
        assert!(verifier.verify(&token_no_facts, None).is_ok());
    }
}
