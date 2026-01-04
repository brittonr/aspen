//! Gossip announcement types for Pijul.
//!
//! This module defines the announcement types broadcast over iroh-gossip
//! to notify peers of new Pijul changes, channel updates, and seeding status.
//!
//! All announcements are signed with Ed25519 for authentication.

use aspen_forge::identity::RepoId;
use iroh::PublicKey;
use iroh::SecretKey;
use iroh::Signature;
use iroh_gossip::proto::TopicId;
use serde::Deserialize;
use serde::Serialize;

use super::types::ChangeHash;

/// Pijul-specific announcement types broadcast over gossip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PijulAnnouncement {
    /// A channel head was updated (new change applied).
    ChannelUpdate {
        /// Repository ID.
        repo_id: RepoId,
        /// Channel name.
        channel: String,
        /// New head change hash.
        new_head: ChangeHash,
        /// Previous head (for detecting divergence).
        old_head: Option<ChangeHash>,
        /// Merkle root after the update.
        merkle: [u8; 32],
    },

    /// A new change is available for a repository.
    ChangeAvailable {
        /// Repository ID.
        repo_id: RepoId,
        /// Hash of the available change.
        change_hash: ChangeHash,
        /// Size in bytes (for transfer planning).
        size_bytes: u32,
        /// Dependencies of this change.
        dependencies: Vec<ChangeHash>,
    },

    /// A node is seeding a Pijul repository.
    Seeding {
        /// Repository ID.
        repo_id: RepoId,
        /// Node ID of the seeder.
        node_id: PublicKey,
        /// Channels this node has.
        channels: Vec<String>,
    },

    /// A node stopped seeding a Pijul repository.
    Unseeding {
        /// Repository ID.
        repo_id: RepoId,
        /// Node ID.
        node_id: PublicKey,
    },

    /// A new Pijul repository was created.
    RepoCreated {
        /// Repository ID.
        repo_id: RepoId,
        /// Repository name.
        name: String,
        /// Default channel name.
        default_channel: String,
        /// Node ID of creator.
        creator: PublicKey,
    },

    /// Request missing changes from peers.
    WantChanges {
        /// Repository ID.
        repo_id: RepoId,
        /// Hashes of changes we want.
        hashes: Vec<ChangeHash>,
        /// Our node ID for responses.
        requester: PublicKey,
    },

    /// Offer changes to a peer (response to WantChanges or proactive).
    HaveChanges {
        /// Repository ID.
        repo_id: RepoId,
        /// Hashes of changes we have.
        hashes: Vec<ChangeHash>,
        /// Our node ID.
        offerer: PublicKey,
    },
}

impl PijulAnnouncement {
    /// Get the repository ID this announcement relates to.
    pub fn repo_id(&self) -> &RepoId {
        match self {
            PijulAnnouncement::ChannelUpdate { repo_id, .. } => repo_id,
            PijulAnnouncement::ChangeAvailable { repo_id, .. } => repo_id,
            PijulAnnouncement::Seeding { repo_id, .. } => repo_id,
            PijulAnnouncement::Unseeding { repo_id, .. } => repo_id,
            PijulAnnouncement::RepoCreated { repo_id, .. } => repo_id,
            PijulAnnouncement::WantChanges { repo_id, .. } => repo_id,
            PijulAnnouncement::HaveChanges { repo_id, .. } => repo_id,
        }
    }

    /// Check if this is a global announcement (goes to global topic).
    pub fn is_global(&self) -> bool {
        matches!(
            self,
            PijulAnnouncement::RepoCreated { .. }
                | PijulAnnouncement::Seeding { .. }
                | PijulAnnouncement::Unseeding { .. }
        )
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

/// Gossip topic for Pijul announcements.
///
/// Topics are scoped by repository to limit message propagation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PijulTopic {
    /// The repository this topic is for, or None for global announcements.
    pub repo_id: Option<RepoId>,
}

impl PijulTopic {
    /// Create a global topic for all Pijul announcements.
    pub fn global() -> Self {
        Self { repo_id: None }
    }

    /// Create a topic scoped to a specific repository.
    pub fn for_repo(repo_id: RepoId) -> Self {
        Self { repo_id: Some(repo_id) }
    }

    /// Convert to topic bytes.
    ///
    /// Uses a different prefix than Forge to avoid topic collision.
    pub fn to_topic_bytes(&self) -> [u8; 32] {
        let data = match &self.repo_id {
            Some(id) => {
                // Hash "pijul:repo:{repo_id}" to get deterministic topic
                let mut buf = Vec::with_capacity(11 + 32);
                buf.extend_from_slice(b"pijul:repo:");
                buf.extend_from_slice(&id.0);
                *blake3::hash(&buf).as_bytes()
            }
            None => *blake3::hash(b"pijul:global").as_bytes(),
        };
        data
    }

    /// Convert to iroh-gossip TopicId.
    pub fn to_topic_id(&self) -> TopicId {
        TopicId::from_bytes(self.to_topic_bytes())
    }
}

/// Signed Pijul announcement for cryptographic verification.
///
/// Wraps a `PijulAnnouncement` with an Ed25519 signature from the sender's
/// secret key. Recipients verify using the embedded public key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedPijulAnnouncement {
    /// The announcement payload.
    pub announcement: PijulAnnouncement,
    /// Ed25519 signature over the serialized announcement.
    pub signature: Signature,
    /// Public key of the signer.
    pub signer: PublicKey,
    /// Timestamp in milliseconds since Unix epoch.
    pub timestamp_ms: u64,
}

impl SignedPijulAnnouncement {
    /// Create and sign an announcement.
    pub fn sign(announcement: PijulAnnouncement, secret_key: &SecretKey) -> Self {
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
    pub fn verify(&self) -> Option<&PijulAnnouncement> {
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

/// Callback trait for receiving Pijul announcements.
///
/// Implement this trait to handle incoming announcements from gossip.
pub trait PijulAnnouncementHandler: Send + Sync {
    /// Called when a verified announcement is received.
    fn on_announcement(&self, announcement: &PijulAnnouncement, signer: &PublicKey);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> SecretKey {
        SecretKey::generate(&mut rand::rng())
    }

    fn test_repo_id() -> RepoId {
        RepoId::from_hash(blake3::hash(b"test-pijul-repo"))
    }

    #[test]
    fn test_announcement_roundtrip() {
        let repo_id = test_repo_id();
        let hash = ChangeHash([1u8; 32]);

        let ann = PijulAnnouncement::ChannelUpdate {
            repo_id,
            channel: "main".to_string(),
            new_head: hash,
            old_head: None,
            merkle: [2u8; 32],
        };

        let bytes = ann.to_bytes();
        let recovered = PijulAnnouncement::from_bytes(&bytes).expect("should deserialize");

        assert_eq!(ann, recovered);
    }

    #[test]
    fn test_topic_bytes_different_from_forge() {
        let pijul_global = PijulTopic::global();
        let forge_global_bytes = *blake3::hash(b"forge:global").as_bytes();

        // Pijul topics should be different from Forge topics
        assert_ne!(pijul_global.to_topic_bytes(), forge_global_bytes);
    }

    #[test]
    fn test_topic_bytes_deterministic() {
        let repo = PijulTopic::for_repo(test_repo_id());
        let repo2 = PijulTopic::for_repo(test_repo_id());

        // Same repo should give same topic
        assert_eq!(repo.to_topic_bytes(), repo2.to_topic_bytes());
    }

    #[test]
    fn test_topic_id_conversion() {
        let global = PijulTopic::global();
        let topic_id = global.to_topic_id();

        assert_eq!(topic_id.as_bytes(), &global.to_topic_bytes());
    }

    #[test]
    fn test_signed_announcement_sign_and_verify() {
        let secret_key = test_key();
        let repo_id = test_repo_id();

        let announcement = PijulAnnouncement::RepoCreated {
            repo_id,
            name: "my-pijul-project".to_string(),
            default_channel: "main".to_string(),
            creator: secret_key.public(),
        };

        let signed = SignedPijulAnnouncement::sign(announcement.clone(), &secret_key);

        // Verify should succeed
        let verified = signed.verify();
        assert!(verified.is_some());
        assert_eq!(verified.unwrap(), &announcement);
        assert_eq!(signed.signer, secret_key.public());
    }

    #[test]
    fn test_signed_announcement_roundtrip() {
        let secret_key = test_key();
        let repo_id = test_repo_id();

        let announcement = PijulAnnouncement::ChannelUpdate {
            repo_id,
            channel: "feature".to_string(),
            new_head: ChangeHash([1u8; 32]),
            old_head: Some(ChangeHash([0u8; 32])),
            merkle: [3u8; 32],
        };

        let signed = SignedPijulAnnouncement::sign(announcement, &secret_key);

        // Serialize and deserialize
        let bytes = signed.to_bytes();
        let recovered = SignedPijulAnnouncement::from_bytes(&bytes).expect("should deserialize");

        // Verify still works after roundtrip
        assert!(recovered.verify().is_some());
        assert_eq!(recovered.signer, signed.signer);
        assert_eq!(recovered.timestamp_ms, signed.timestamp_ms);
    }

    #[test]
    fn test_signed_announcement_wrong_key_rejected() {
        let real_key = test_key();
        let fake_key = test_key();
        let repo_id = test_repo_id();

        let announcement = PijulAnnouncement::Seeding {
            repo_id,
            node_id: real_key.public(),
            channels: vec!["main".to_string()],
        };

        // Sign with real key but claim different signer
        let mut signed = SignedPijulAnnouncement::sign(announcement, &real_key);
        signed.signer = fake_key.public(); // Tamper with signer

        // Verification should fail
        assert!(signed.verify().is_none());
    }

    #[test]
    fn test_signed_announcement_tampered_rejected() {
        let secret_key = test_key();
        let repo_id = test_repo_id();

        let announcement = PijulAnnouncement::ChangeAvailable {
            repo_id,
            change_hash: ChangeHash([1u8; 32]),
            size_bytes: 1024,
            dependencies: vec![],
        };

        let mut signed = SignedPijulAnnouncement::sign(announcement, &secret_key);

        // Tamper with the announcement
        if let PijulAnnouncement::ChangeAvailable { ref mut size_bytes, .. } = signed.announcement {
            *size_bytes = 9999;
        }

        // Verification should fail
        assert!(signed.verify().is_none());
    }

    #[test]
    fn test_is_global() {
        let repo_id = test_repo_id();
        let key = test_key();

        // Global announcements
        assert!(
            PijulAnnouncement::RepoCreated {
                repo_id,
                name: "test".to_string(),
                default_channel: "main".to_string(),
                creator: key.public(),
            }
            .is_global()
        );

        assert!(
            PijulAnnouncement::Seeding {
                repo_id,
                node_id: key.public(),
                channels: vec![],
            }
            .is_global()
        );

        assert!(
            PijulAnnouncement::Unseeding {
                repo_id,
                node_id: key.public(),
            }
            .is_global()
        );

        // Non-global announcements
        assert!(
            !PijulAnnouncement::ChannelUpdate {
                repo_id,
                channel: "main".to_string(),
                new_head: ChangeHash([0u8; 32]),
                old_head: None,
                merkle: [0u8; 32],
            }
            .is_global()
        );

        assert!(
            !PijulAnnouncement::ChangeAvailable {
                repo_id,
                change_hash: ChangeHash([0u8; 32]),
                size_bytes: 100,
                dependencies: vec![],
            }
            .is_global()
        );
    }

    #[test]
    fn test_all_announcement_types() {
        let key = test_key();
        let repo_id = test_repo_id();

        // Test all announcement variants can be signed and verified
        let announcements = vec![
            PijulAnnouncement::ChannelUpdate {
                repo_id,
                channel: "main".to_string(),
                new_head: ChangeHash([1u8; 32]),
                old_head: None,
                merkle: [2u8; 32],
            },
            PijulAnnouncement::ChangeAvailable {
                repo_id,
                change_hash: ChangeHash([3u8; 32]),
                size_bytes: 512,
                dependencies: vec![ChangeHash([4u8; 32])],
            },
            PijulAnnouncement::Seeding {
                repo_id,
                node_id: key.public(),
                channels: vec!["main".to_string(), "develop".to_string()],
            },
            PijulAnnouncement::Unseeding {
                repo_id,
                node_id: key.public(),
            },
            PijulAnnouncement::RepoCreated {
                repo_id,
                name: "test".to_string(),
                default_channel: "main".to_string(),
                creator: key.public(),
            },
            PijulAnnouncement::WantChanges {
                repo_id,
                hashes: vec![ChangeHash([5u8; 32]), ChangeHash([6u8; 32])],
                requester: key.public(),
            },
            PijulAnnouncement::HaveChanges {
                repo_id,
                hashes: vec![ChangeHash([7u8; 32])],
                offerer: key.public(),
            },
        ];

        for announcement in announcements {
            let signed = SignedPijulAnnouncement::sign(announcement.clone(), &key);
            let verified = signed.verify();
            assert!(verified.is_some());
            assert_eq!(verified.unwrap(), &announcement);
        }
    }
}
