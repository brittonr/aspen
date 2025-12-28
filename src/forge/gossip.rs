//! Gossip announcements for Forge.
//!
//! This module defines the announcement types broadcast over iroh-gossip
//! to notify peers of new commits, COB changes, and other updates.

use iroh::PublicKey;
use serde::{Deserialize, Serialize};

use crate::forge::cob::CobType;
use crate::forge::identity::RepoId;

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
                let mut buf = [0u8; 64];
                buf[..32].copy_from_slice(b"forge:repo:");
                buf[11..43].copy_from_slice(&id.0);
                blake3::hash(&buf).into()
            }
            None => *blake3::hash(b"forge:global").as_bytes(),
        };
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
