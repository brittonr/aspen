//! Gossip announcement types for Forge.
//!
//! This module defines the announcement types broadcast over iroh-gossip
//! to notify peers of new commits, COB changes, and other updates.
//!
//! All announcements are signed with Ed25519 for authentication.

use iroh::{PublicKey, SecretKey, Signature};
use iroh_gossip::proto::TopicId;
use serde::{Deserialize, Serialize};

use crate::cob::CobType;
use crate::identity::RepoId;

/// Announcement types broadcast over gossip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Announcement {
    /// A ref was updated (new commit pushed).
    RefUpdate {
        /// Repository ID.
        repo_id: RepoId,
        /// Ref name (e.g., "heads/main").
        ref_name: String,
        /// New hash the ref points to.
        new_hash: [u8; 32],
        /// Previous hash (for fast-forward detection).
        old_hash: Option<[u8; 32]>,
    },

    /// A new COB change was created.
    CobChange {
        /// Repository ID.
        repo_id: RepoId,
        /// COB type.
        cob_type: CobType,
        /// COB instance ID.
        cob_id: [u8; 32],
        /// Hash of the new change.
        change_hash: [u8; 32],
    },

    /// A node is seeding a repository.
    Seeding {
        /// Repository ID.
        repo_id: RepoId,
        /// Node ID of the seeder.
        node_id: PublicKey,
    },

    /// A node stopped seeding a repository.
    Unseeding {
        /// Repository ID.
        repo_id: RepoId,
        /// Node ID.
        node_id: PublicKey,
    },

    /// A new repository was created.
    RepoCreated {
        /// Repository ID.
        repo_id: RepoId,
        /// Repository name.
        name: String,
        /// Node ID of creator.
        creator: PublicKey,
    },
}

impl Announcement {
    /// Get the repository ID this announcement relates to.
    pub fn repo_id(&self) -> &RepoId {
        match self {
            Announcement::RefUpdate { repo_id, .. } => repo_id,
            Announcement::CobChange { repo_id, .. } => repo_id,
            Announcement::Seeding { repo_id, .. } => repo_id,
            Announcement::Unseeding { repo_id, .. } => repo_id,
            Announcement::RepoCreated { repo_id, .. } => repo_id,
        }
    }

    /// Serialize to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        postcard::to_allocvec(self).expect("serialization should not fail")
    }

    /// Deserialize from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(bytes)
    }
}

/// Gossip topic for Forge announcements.
///
/// Topics are scoped by repository to limit message propagation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ForgeTopic {
    /// The repository this topic is for, or None for global announcements.
    pub repo_id: Option<RepoId>,
}

impl ForgeTopic {
    /// Create a global topic for all Forge announcements.
    pub fn global() -> Self {
        Self { repo_id: None }
    }

    /// Create a topic scoped to a specific repository.
    pub fn for_repo(repo_id: RepoId) -> Self {
        Self {
            repo_id: Some(repo_id),
        }
    }

    /// Convert to iroh-gossip topic bytes.
    pub fn to_topic_bytes(&self) -> [u8; 32] {
        let data = match &self.repo_id {
            Some(id) => {
                // Hash "forge:repo:{repo_id}" to get deterministic topic
                let mut buf = Vec::with_capacity(11 + 32);
                buf.extend_from_slice(b"forge:repo:");
                buf.extend_from_slice(&id.0);
                *blake3::hash(&buf).as_bytes()
            }
            None => *blake3::hash(b"forge:global").as_bytes(),
        };
        data
    }

    /// Convert to iroh-gossip TopicId.
    pub fn to_topic_id(&self) -> TopicId {
        TopicId::from_bytes(self.to_topic_bytes())
    }
}

/// Signed announcement for cryptographic verification.
///
/// Wraps an `Announcement` with an Ed25519 signature from the sender's
/// secret key. Recipients verify using the embedded public key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedAnnouncement {
    /// The announcement payload.
    pub announcement: Announcement,
    /// Ed25519 signature over the serialized announcement.
    pub signature: Signature,
    /// Public key of the signer.
    pub signer: PublicKey,
    /// Timestamp in milliseconds since Unix epoch.
    pub timestamp_ms: u64,
}

impl SignedAnnouncement {
    /// Create and sign an announcement.
    pub fn sign(announcement: Announcement, secret_key: &SecretKey) -> Self {
        let announcement_bytes = announcement.to_bytes();
        let signature = secret_key.sign(&announcement_bytes);
        let signer = secret_key.public();
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_millis() as u64;

        Self {
            announcement,
            signature,
            signer,
            timestamp_ms,
        }
    }

    /// Verify the signature and return the inner announcement if valid.
    pub fn verify(&self) -> Option<&Announcement> {
        let announcement_bytes = self.announcement.to_bytes();

        match self.signer.verify(&announcement_bytes, &self.signature) {
            Ok(()) => Some(&self.announcement),
            Err(_) => None,
        }
    }

    /// Serialize to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        postcard::to_allocvec(self).expect("serialization should not fail")
    }

    /// Deserialize from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(bytes)
    }
}

/// Callback trait for receiving announcements.
///
/// Implement this trait to handle incoming announcements from gossip.
pub trait AnnouncementHandler: Send + Sync {
    /// Called when a verified announcement is received.
    fn on_announcement(&self, announcement: &Announcement, signer: &PublicKey);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> SecretKey {
        SecretKey::generate(&mut rand::rng())
    }

    #[test]
    fn test_announcement_roundtrip() {
        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));
        let hash = *blake3::hash(b"commit").as_bytes();

        let ann = Announcement::RefUpdate {
            repo_id,
            ref_name: "heads/main".to_string(),
            new_hash: hash,
            old_hash: None,
        };

        let bytes = ann.to_bytes();
        let recovered = Announcement::from_bytes(&bytes).expect("should deserialize");

        assert_eq!(ann, recovered);
    }

    #[test]
    fn test_topic_bytes() {
        let global = ForgeTopic::global();
        let repo = ForgeTopic::for_repo(RepoId::from_hash(blake3::hash(b"test")));

        // Topics should be different
        assert_ne!(global.to_topic_bytes(), repo.to_topic_bytes());

        // Same repo should give same topic
        let repo2 = ForgeTopic::for_repo(RepoId::from_hash(blake3::hash(b"test")));
        assert_eq!(repo.to_topic_bytes(), repo2.to_topic_bytes());
    }

    #[test]
    fn test_topic_id_conversion() {
        let global = ForgeTopic::global();
        let topic_id = global.to_topic_id();

        // TopicId should be derived from topic bytes
        assert_eq!(topic_id.as_bytes(), &global.to_topic_bytes());
    }

    #[test]
    fn test_signed_announcement_sign_and_verify() {
        let secret_key = test_key();
        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));

        let announcement = Announcement::RepoCreated {
            repo_id,
            name: "my-project".to_string(),
            creator: secret_key.public(),
        };

        let signed = SignedAnnouncement::sign(announcement.clone(), &secret_key);

        // Verify should succeed
        let verified = signed.verify();
        assert!(verified.is_some());
        assert_eq!(verified.unwrap(), &announcement);
        assert_eq!(signed.signer, secret_key.public());
    }

    #[test]
    fn test_signed_announcement_roundtrip() {
        let secret_key = test_key();
        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));

        let announcement = Announcement::RefUpdate {
            repo_id,
            ref_name: "heads/main".to_string(),
            new_hash: [1u8; 32],
            old_hash: Some([0u8; 32]),
        };

        let signed = SignedAnnouncement::sign(announcement, &secret_key);

        // Serialize and deserialize
        let bytes = signed.to_bytes();
        let recovered = SignedAnnouncement::from_bytes(&bytes).expect("should deserialize");

        // Verify still works after roundtrip
        assert!(recovered.verify().is_some());
        assert_eq!(recovered.signer, signed.signer);
        assert_eq!(recovered.timestamp_ms, signed.timestamp_ms);
    }

    #[test]
    fn test_signed_announcement_wrong_key_rejected() {
        let real_key = test_key();
        let fake_key = test_key();
        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));

        let announcement = Announcement::Seeding {
            repo_id,
            node_id: real_key.public(),
        };

        // Sign with real key but claim different signer
        let mut signed = SignedAnnouncement::sign(announcement, &real_key);
        signed.signer = fake_key.public(); // Tamper with signer

        // Verification should fail
        assert!(signed.verify().is_none());
    }

    #[test]
    fn test_signed_announcement_tampered_rejected() {
        let secret_key = test_key();
        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));

        let announcement = Announcement::CobChange {
            repo_id,
            cob_type: CobType::Issue,
            cob_id: [1u8; 32],
            change_hash: [2u8; 32],
        };

        let mut signed = SignedAnnouncement::sign(announcement, &secret_key);

        // Tamper with the announcement
        if let Announcement::CobChange { ref mut cob_id, .. } = signed.announcement {
            *cob_id = [99u8; 32];
        }

        // Verification should fail
        assert!(signed.verify().is_none());
    }

    #[test]
    fn test_all_announcement_types() {
        let key = test_key();
        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));

        // Test all announcement variants can be signed and verified
        let announcements = vec![
            Announcement::RefUpdate {
                repo_id,
                ref_name: "heads/main".to_string(),
                new_hash: [1u8; 32],
                old_hash: None,
            },
            Announcement::CobChange {
                repo_id,
                cob_type: CobType::Patch,
                cob_id: [2u8; 32],
                change_hash: [3u8; 32],
            },
            Announcement::Seeding {
                repo_id,
                node_id: key.public(),
            },
            Announcement::Unseeding {
                repo_id,
                node_id: key.public(),
            },
            Announcement::RepoCreated {
                repo_id,
                name: "test".to_string(),
                creator: key.public(),
            },
        ];

        for announcement in announcements {
            let signed = SignedAnnouncement::sign(announcement.clone(), &key);
            let verified = signed.verify();
            assert!(verified.is_some());
            assert_eq!(verified.unwrap(), &announcement);
        }
    }
}
