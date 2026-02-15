//! Delegate signature verification for canonical refs.
//!
//! This module provides authorization checking for ref updates,
//! ensuring that canonical refs (like heads/main) are only updated
//! by authorized delegates.

use iroh::PublicKey;
use iroh::SecretKey;
use iroh::Signature;
use serde::Deserialize;
use serde::Serialize;

use crate::error::ForgeError;
use crate::error::ForgeResult;
use crate::identity::RepoId;
use crate::identity::RepoIdentity;

/// A signed ref update request.
///
/// Contains the update details and a signature from the signer.
/// For canonical refs, the signer must be an authorized delegate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedRefUpdate {
    /// Repository ID.
    pub repo_id: RepoId,
    /// Ref name (e.g., "heads/main").
    pub ref_name: String,
    /// New hash value.
    pub new_hash: [u8; 32],
    /// Previous hash (None for create).
    pub old_hash: Option<[u8; 32]>,
    /// Signer's public key.
    pub signer: PublicKey,
    /// Signature over the update data.
    pub signature: Signature,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
}

impl SignedRefUpdate {
    /// Create and sign a ref update.
    pub fn sign(
        repo_id: RepoId,
        ref_name: impl Into<String>,
        new_hash: blake3::Hash,
        old_hash: Option<blake3::Hash>,
        secret_key: &SecretKey,
    ) -> Self {
        let ref_name = ref_name.into();
        let new_hash_bytes = *new_hash.as_bytes();
        let old_hash_bytes = old_hash.map(|h| *h.as_bytes());

        // Build the message to sign
        let message = Self::build_message(&repo_id, &ref_name, &new_hash_bytes, &old_hash_bytes);
        let signature = secret_key.sign(&message);
        let signer = secret_key.public();
        let timestamp_ms =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

        Self {
            repo_id,
            ref_name,
            new_hash: new_hash_bytes,
            old_hash: old_hash_bytes,
            signer,
            signature,
            timestamp_ms,
        }
    }

    /// Verify the signature is valid.
    pub fn verify_signature(&self) -> bool {
        let message = Self::build_message(&self.repo_id, &self.ref_name, &self.new_hash, &self.old_hash);
        self.signer.verify(&message, &self.signature).is_ok()
    }

    /// Build the message bytes for signing/verification.
    fn build_message(repo_id: &RepoId, ref_name: &str, new_hash: &[u8; 32], old_hash: &Option<[u8; 32]>) -> Vec<u8> {
        let mut message = Vec::with_capacity(32 + ref_name.len() + 32 + 33);
        message.extend_from_slice(&repo_id.0);
        message.extend_from_slice(ref_name.as_bytes());
        message.extend_from_slice(new_hash);
        if let Some(old) = old_hash {
            message.push(1);
            message.extend_from_slice(old);
        } else {
            message.push(0);
        }
        message
    }

    /// Get the new hash as a blake3::Hash.
    pub fn new_hash(&self) -> blake3::Hash {
        blake3::Hash::from_bytes(self.new_hash)
    }

    /// Get the old hash as a blake3::Hash.
    pub fn old_hash(&self) -> Option<blake3::Hash> {
        self.old_hash.map(blake3::Hash::from_bytes)
    }
}

/// Delegate verifier for checking ref update authorization.
///
/// Determines if a signer is authorized to update a specific ref.
pub struct DelegateVerifier;

impl DelegateVerifier {
    /// Check if a ref is a canonical ref that requires delegate authorization.
    ///
    /// Canonical refs are:
    /// - `heads/{default_branch}` (e.g., "heads/main")
    /// - `tags/*` (all tags)
    ///
    /// Non-canonical refs (contributor refs) don't require delegate auth:
    /// - `heads/{node_id}/*` (contributor branches)
    pub fn is_canonical_ref(ref_name: &str, default_branch: &str) -> bool {
        // The default branch is canonical
        if ref_name == format!("heads/{}", default_branch) {
            return true;
        }

        // All tags are canonical
        if ref_name.starts_with("tags/") {
            return true;
        }

        // Protected refs are canonical
        if ref_name.starts_with("refs/") {
            return true;
        }

        // Other heads/* refs are contributor refs (non-canonical)
        false
    }

    /// Verify that a signed ref update is authorized.
    ///
    /// For canonical refs, the signer must be a delegate of the repository.
    /// For non-canonical refs, any valid signature is accepted.
    ///
    /// # Errors
    ///
    /// - `ForgeError::NotAuthorized` if the signer is not a delegate for canonical refs
    /// - `ForgeError::InvalidSignature` if the signature is invalid
    pub fn verify_update(update: &SignedRefUpdate, identity: &RepoIdentity) -> ForgeResult<()> {
        // First, verify the signature is valid
        if !update.verify_signature() {
            return Err(ForgeError::InvalidSignature {
                message: "ref update signature verification failed".to_string(),
            });
        }

        // Check if this is a canonical ref
        if Self::is_canonical_ref(&update.ref_name, &identity.default_branch) {
            // Canonical refs require delegate authorization
            if !identity.is_delegate(&update.signer) {
                return Err(ForgeError::NotAuthorized {
                    signer: update.signer.to_string(),
                    action: format!("update canonical ref '{}'", update.ref_name),
                });
            }
        }

        // For non-canonical refs, any valid signature is accepted
        Ok(())
    }

    /// Check if a signer can update a specific ref (without requiring a full SignedRefUpdate).
    ///
    /// This is useful for pre-flight checks before creating a signed update.
    pub fn can_update_ref(signer: &PublicKey, ref_name: &str, identity: &RepoIdentity) -> bool {
        if Self::is_canonical_ref(ref_name, &identity.default_branch) {
            identity.is_delegate(signer)
        } else {
            // Anyone can update non-canonical refs
            true
        }
    }
}

/// Multi-signature collector for threshold-based authorization.
///
/// Collects signatures from multiple delegates until the threshold is reached.
#[derive(Debug, Clone, Default)]
pub struct MultiSigCollector {
    /// Collected signatures (signer -> signature).
    signatures: Vec<(PublicKey, Signature)>,
    /// The message being signed.
    message: Vec<u8>,
}

impl MultiSigCollector {
    /// Create a new collector for a ref update.
    pub fn new(repo_id: &RepoId, ref_name: &str, new_hash: &[u8; 32], old_hash: &Option<[u8; 32]>) -> Self {
        let message = SignedRefUpdate::build_message(repo_id, ref_name, new_hash, old_hash);
        Self {
            signatures: Vec::new(),
            message,
        }
    }

    /// Add a signature from a delegate.
    ///
    /// Returns true if the signature is valid and was added.
    pub fn add_signature(&mut self, signer: PublicKey, signature: Signature) -> bool {
        // Verify signature
        if signer.verify(&self.message, &signature).is_err() {
            return false;
        }

        // Check for duplicate signer
        if self.signatures.iter().any(|(s, _)| s == &signer) {
            return false;
        }

        self.signatures.push((signer, signature));
        true
    }

    /// Check if we have enough signatures to meet the threshold.
    pub fn has_quorum(&self, identity: &RepoIdentity) -> bool {
        let valid_delegates: u32 =
            self.signatures.iter().filter(|(signer, _)| identity.is_delegate(signer)).count() as u32;

        valid_delegates >= identity.threshold
    }

    /// Get the number of valid delegate signatures collected.
    pub fn delegate_count(&self, identity: &RepoIdentity) -> u32 {
        self.signatures.iter().filter(|(signer, _)| identity.is_delegate(signer)).count() as u32
    }

    /// Get all collected signatures.
    pub fn signatures(&self) -> &[(PublicKey, Signature)] {
        &self.signatures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> SecretKey {
        SecretKey::generate(&mut rand::rng())
    }

    fn test_identity(delegates: Vec<PublicKey>, threshold: u32) -> RepoIdentity {
        RepoIdentity::new("test-repo", delegates, threshold).unwrap()
    }

    #[test]
    fn test_signed_ref_update_roundtrip() {
        let secret_key = test_key();
        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));
        let new_hash = blake3::hash(b"new-commit");
        let old_hash = Some(blake3::hash(b"old-commit"));

        let update = SignedRefUpdate::sign(repo_id, "heads/main", new_hash, old_hash, &secret_key);

        assert!(update.verify_signature());
        assert_eq!(update.new_hash(), new_hash);
        assert_eq!(update.old_hash(), old_hash);
        assert_eq!(update.signer, secret_key.public());
    }

    #[test]
    fn test_signed_ref_update_tampered() {
        let secret_key = test_key();
        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));
        let new_hash = blake3::hash(b"new-commit");

        let mut update = SignedRefUpdate::sign(repo_id, "heads/main", new_hash, None, &secret_key);

        // Tamper with the hash
        update.new_hash = [99u8; 32];

        // Signature should fail
        assert!(!update.verify_signature());
    }

    #[test]
    fn test_is_canonical_ref() {
        // Default branch is canonical
        assert!(DelegateVerifier::is_canonical_ref("heads/main", "main"));
        assert!(DelegateVerifier::is_canonical_ref("heads/master", "master"));

        // Tags are canonical
        assert!(DelegateVerifier::is_canonical_ref("tags/v1.0.0", "main"));
        assert!(DelegateVerifier::is_canonical_ref("tags/release", "main"));

        // Other branches are NOT canonical
        assert!(!DelegateVerifier::is_canonical_ref("heads/feature", "main"));
        assert!(!DelegateVerifier::is_canonical_ref("heads/fix-bug", "main"));
        assert!(!DelegateVerifier::is_canonical_ref("heads/abc123/feature", "main"));
    }

    #[test]
    fn test_verify_update_delegate_authorized() {
        let delegate_key = test_key();
        let identity = test_identity(vec![delegate_key.public()], 1);
        let repo_id = identity.repo_id();
        let new_hash = blake3::hash(b"commit");

        // Delegate updating canonical ref should succeed
        let update = SignedRefUpdate::sign(repo_id, "heads/main", new_hash, None, &delegate_key);

        assert!(DelegateVerifier::verify_update(&update, &identity).is_ok());
    }

    #[test]
    fn test_verify_update_non_delegate_canonical() {
        let delegate_key = test_key();
        let non_delegate_key = test_key();
        let identity = test_identity(vec![delegate_key.public()], 1);
        let repo_id = identity.repo_id();
        let new_hash = blake3::hash(b"commit");

        // Non-delegate updating canonical ref should fail
        let update = SignedRefUpdate::sign(repo_id, "heads/main", new_hash, None, &non_delegate_key);

        let result = DelegateVerifier::verify_update(&update, &identity);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ForgeError::NotAuthorized { .. }));
    }

    #[test]
    fn test_verify_update_non_delegate_contributor_ref() {
        let delegate_key = test_key();
        let contributor_key = test_key();
        let identity = test_identity(vec![delegate_key.public()], 1);
        let repo_id = identity.repo_id();
        let new_hash = blake3::hash(b"commit");

        // Non-delegate updating contributor ref should succeed
        let update = SignedRefUpdate::sign(repo_id, "heads/feature-branch", new_hash, None, &contributor_key);

        assert!(DelegateVerifier::verify_update(&update, &identity).is_ok());
    }

    #[test]
    fn test_can_update_ref() {
        let delegate_key = test_key();
        let contributor_key = test_key();
        let identity = test_identity(vec![delegate_key.public()], 1);

        // Delegate can update any ref
        assert!(DelegateVerifier::can_update_ref(&delegate_key.public(), "heads/main", &identity));
        assert!(DelegateVerifier::can_update_ref(&delegate_key.public(), "tags/v1.0", &identity));
        assert!(DelegateVerifier::can_update_ref(&delegate_key.public(), "heads/feature", &identity));

        // Contributor can only update non-canonical refs
        assert!(!DelegateVerifier::can_update_ref(&contributor_key.public(), "heads/main", &identity));
        assert!(!DelegateVerifier::can_update_ref(&contributor_key.public(), "tags/v1.0", &identity));
        assert!(DelegateVerifier::can_update_ref(&contributor_key.public(), "heads/feature", &identity));
    }

    #[test]
    fn test_multi_sig_collector() {
        let key1 = test_key();
        let key2 = test_key();
        let key3 = test_key();
        let identity = test_identity(vec![key1.public(), key2.public(), key3.public()], 2);
        let repo_id = identity.repo_id();
        let new_hash = *blake3::hash(b"commit").as_bytes();

        let mut collector = MultiSigCollector::new(&repo_id, "heads/main", &new_hash, &None);

        // Build the message manually for signing
        let message = SignedRefUpdate::build_message(&repo_id, "heads/main", &new_hash, &None);

        // Add first signature
        let sig1 = key1.sign(&message);
        assert!(collector.add_signature(key1.public(), sig1));
        assert!(!collector.has_quorum(&identity)); // 1 < 2

        // Add second signature
        let sig2 = key2.sign(&message);
        assert!(collector.add_signature(key2.public(), sig2));
        assert!(collector.has_quorum(&identity)); // 2 >= 2

        assert_eq!(collector.delegate_count(&identity), 2);
    }

    #[test]
    fn test_multi_sig_rejects_invalid() {
        let key1 = test_key();
        let key2 = test_key();
        let identity = test_identity(vec![key1.public()], 1);
        let repo_id = identity.repo_id();
        let new_hash = *blake3::hash(b"commit").as_bytes();

        let mut collector = MultiSigCollector::new(&repo_id, "heads/main", &new_hash, &None);

        // Try to add signature with wrong message
        let wrong_sig = key1.sign(b"wrong message");
        assert!(!collector.add_signature(key1.public(), wrong_sig));

        // Collector should be empty
        assert_eq!(collector.signatures().len(), 0);
    }

    #[test]
    fn test_multi_sig_rejects_duplicates() {
        let key1 = test_key();
        let identity = test_identity(vec![key1.public()], 1);
        let repo_id = identity.repo_id();
        let new_hash = *blake3::hash(b"commit").as_bytes();

        let mut collector = MultiSigCollector::new(&repo_id, "heads/main", &new_hash, &None);
        let message = SignedRefUpdate::build_message(&repo_id, "heads/main", &new_hash, &None);

        // Add signature once
        let sig = key1.sign(&message);
        assert!(collector.add_signature(key1.public(), sig.clone()));

        // Try to add same signer again
        assert!(!collector.add_signature(key1.public(), sig));

        assert_eq!(collector.signatures().len(), 1);
    }
}
