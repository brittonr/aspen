//! Capability definitions for authorization.
//!
//! Capabilities represent what operations a token holder can perform.
//! They follow the principle of least privilege with prefix-based scoping.

use serde::{Deserialize, Serialize};

/// What operations a token holder can perform.
///
/// Capabilities are scoped by key prefix - a capability for prefix "users:"
/// authorizes operations on any key starting with "users:".
///
/// # Tiger Style
///
/// - Explicit variants for each operation type
/// - No regex or complex patterns - prefix matching only
/// - ClusterAdmin is separate from data operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Read keys matching this prefix.
    Read {
        /// Key prefix this capability applies to.
        prefix: String,
    },
    /// Write keys matching this prefix.
    Write {
        /// Key prefix this capability applies to.
        prefix: String,
    },
    /// Delete keys matching this prefix.
    Delete {
        /// Key prefix this capability applies to.
        prefix: String,
    },
    /// Read + Write + Delete for prefix (convenience variant).
    Full {
        /// Key prefix this capability applies to.
        prefix: String,
    },
    /// Subscribe to watch events for prefix.
    Watch {
        /// Key prefix this capability applies to.
        prefix: String,
    },
    /// Cluster admin operations (membership, snapshots).
    ClusterAdmin,
    /// Can create child tokens (with attenuation).
    Delegate,
}

impl Capability {
    /// Check if this capability authorizes the given operation.
    ///
    /// Returns true if this capability grants permission for the operation.
    pub fn authorizes(&self, op: &Operation) -> bool {
        match (self, op) {
            // Full authorizes all data operations for matching prefix
            (Capability::Full { prefix }, Operation::Read { key }) => key.starts_with(prefix),
            (Capability::Full { prefix }, Operation::Write { key, .. }) => key.starts_with(prefix),
            (Capability::Full { prefix }, Operation::Delete { key }) => key.starts_with(prefix),
            (Capability::Full { prefix }, Operation::Watch { key_prefix }) => {
                key_prefix.starts_with(prefix)
            }
            // Specific capabilities
            (Capability::Read { prefix }, Operation::Read { key }) => key.starts_with(prefix),
            (Capability::Write { prefix }, Operation::Write { key, .. }) => key.starts_with(prefix),
            (Capability::Delete { prefix }, Operation::Delete { key }) => key.starts_with(prefix),
            (Capability::Watch { prefix }, Operation::Watch { key_prefix }) => {
                key_prefix.starts_with(prefix)
            }
            // ClusterAdmin operations
            (Capability::ClusterAdmin, Operation::ClusterAdmin { .. }) => true,
            // No match
            _ => false,
        }
    }

    /// Check if this capability is a superset of another (for delegation).
    ///
    /// During delegation, a child token can only have capabilities that are
    /// subsets of the parent's capabilities. This prevents privilege escalation.
    ///
    /// Returns true if `self` contains `other`.
    pub fn contains(&self, other: &Capability) -> bool {
        match (self, other) {
            // Full contains Read, Write, Delete, Watch for same or narrower prefix
            (Capability::Full { prefix: p1 }, Capability::Read { prefix: p2 }) => {
                p2.starts_with(p1)
            }
            (Capability::Full { prefix: p1 }, Capability::Write { prefix: p2 }) => {
                p2.starts_with(p1)
            }
            (Capability::Full { prefix: p1 }, Capability::Delete { prefix: p2 }) => {
                p2.starts_with(p1)
            }
            (Capability::Full { prefix: p1 }, Capability::Watch { prefix: p2 }) => {
                p2.starts_with(p1)
            }
            (Capability::Full { prefix: p1 }, Capability::Full { prefix: p2 }) => {
                p2.starts_with(p1)
            }
            // Same type with narrower prefix
            (Capability::Read { prefix: p1 }, Capability::Read { prefix: p2 }) => {
                p2.starts_with(p1)
            }
            (Capability::Write { prefix: p1 }, Capability::Write { prefix: p2 }) => {
                p2.starts_with(p1)
            }
            (Capability::Delete { prefix: p1 }, Capability::Delete { prefix: p2 }) => {
                p2.starts_with(p1)
            }
            (Capability::Watch { prefix: p1 }, Capability::Watch { prefix: p2 }) => {
                p2.starts_with(p1)
            }
            // ClusterAdmin only contains itself
            (Capability::ClusterAdmin, Capability::ClusterAdmin) => true,
            // Delegate only contains itself
            (Capability::Delegate, Capability::Delegate) => true,
            _ => false,
        }
    }
}

/// Operations that require authorization.
///
/// These map to the client RPC operations that need capability checks.
#[derive(Debug, Clone)]
pub enum Operation {
    /// Read a key.
    Read {
        /// Key to read.
        key: String,
    },
    /// Write a key-value pair.
    Write {
        /// Key to write.
        key: String,
        /// Value to write (for logging/auditing).
        value: Vec<u8>,
    },
    /// Delete a key.
    Delete {
        /// Key to delete.
        key: String,
    },
    /// Watch keys with prefix.
    Watch {
        /// Key prefix to watch.
        key_prefix: String,
    },
    /// Cluster admin operation.
    ClusterAdmin {
        /// Description of the action (for logging).
        action: String,
    },
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Read { key } => write!(f, "Read({})", key),
            Operation::Write { key, .. } => write!(f, "Write({})", key),
            Operation::Delete { key } => write!(f, "Delete({})", key),
            Operation::Watch { key_prefix } => write!(f, "Watch({})", key_prefix),
            Operation::ClusterAdmin { action } => write!(f, "ClusterAdmin({})", action),
        }
    }
}
