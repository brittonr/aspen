//! Capability definitions for authorization.
//!
//! Capabilities represent what operations a token holder can perform.
//! They follow the principle of least privilege with prefix-based scoping.

use serde::Deserialize;
use serde::Serialize;

/// Simple glob pattern matching for shell command authorization.
///
/// Supports only `*` wildcards at the end of patterns (e.g., "pg_*").
/// Returns true if the pattern matches the input.
fn glob_match(pattern: &str, input: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if let Some(prefix) = pattern.strip_suffix('*') {
        // Pattern like "pg_*" matches anything starting with "pg_"
        input.starts_with(prefix)
    } else if pattern.contains('*') {
        // More complex patterns: split by * and check each segment
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.is_empty() {
            return true;
        }

        let mut remaining = input;

        // First part must be at the start
        if !parts[0].is_empty() && !remaining.starts_with(parts[0]) {
            return false;
        }
        remaining = &remaining[parts[0].len()..];

        // Middle parts must exist somewhere in order
        for part in parts.iter().skip(1).take(parts.len().saturating_sub(2)) {
            if part.is_empty() {
                continue;
            }
            if let Some(pos) = remaining.find(part) {
                remaining = &remaining[pos + part.len()..];
            } else {
                return false;
            }
        }

        // Last part must be at the end (if non-empty)
        if let Some(last) = parts.last()
            && !last.is_empty()
            && !remaining.ends_with(last)
        {
            return false;
        }

        true
    } else {
        // No wildcards, exact match
        pattern == input
    }
}

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
    /// Execute shell commands matching a pattern.
    ///
    /// Pattern supports:
    /// - Exact command name: `"goose-cli"`
    /// - Glob patterns: `"pg_*"` (matches pg_dump, pg_restore, etc.)
    /// - Wildcard: `"*"` (all commands - use with caution)
    ///
    /// Working directory can optionally constrain where commands execute.
    ShellExecute {
        /// Command pattern (exact name, glob, or "*").
        command_pattern: String,
        /// Optional working directory constraint (prefix match).
        working_dir: Option<String>,
    },

    // ==========================================================================
    // Secrets Engine Capabilities
    // ==========================================================================
    /// Read secrets from a mount path prefix.
    SecretsRead {
        /// Mount path (e.g., "secret/", "kv/").
        mount: String,
        /// Key prefix within the mount.
        prefix: String,
    },
    /// Write secrets to a mount path prefix.
    SecretsWrite {
        /// Mount path.
        mount: String,
        /// Key prefix within the mount.
        prefix: String,
    },
    /// Delete secrets from a mount path prefix.
    SecretsDelete {
        /// Mount path.
        mount: String,
        /// Key prefix within the mount.
        prefix: String,
    },
    /// List secrets at a mount path prefix.
    SecretsList {
        /// Mount path.
        mount: String,
        /// Key prefix within the mount.
        prefix: String,
    },
    /// Full access (Read + Write + Delete + List) for secrets.
    SecretsFull {
        /// Mount path.
        mount: String,
        /// Key prefix within the mount.
        prefix: String,
    },

    // ==========================================================================
    // Transit Engine Capabilities
    // ==========================================================================
    /// Encrypt data using transit keys.
    TransitEncrypt {
        /// Transit key name prefix (e.g., "app-*" matches "app-db", "app-api").
        key_prefix: String,
    },
    /// Decrypt data using transit keys.
    TransitDecrypt {
        /// Transit key name prefix.
        key_prefix: String,
    },
    /// Sign data using transit keys.
    TransitSign {
        /// Transit key name prefix.
        key_prefix: String,
    },
    /// Verify signatures using transit keys.
    TransitVerify {
        /// Transit key name prefix.
        key_prefix: String,
    },
    /// Manage transit keys (create, rotate, delete, configure).
    TransitKeyManage {
        /// Transit key name prefix.
        key_prefix: String,
    },

    // ==========================================================================
    // PKI Engine Capabilities
    // ==========================================================================
    /// Issue certificates using PKI roles.
    PkiIssue {
        /// Role name prefix (e.g., "web-*" matches "web-server", "web-client").
        role_prefix: String,
    },
    /// Revoke issued certificates.
    PkiRevoke,
    /// Read CA certificate and chain.
    PkiReadCa,
    /// Manage PKI (configure CA, roles, CRL).
    PkiManage,

    // ==========================================================================
    // Secrets Admin
    // ==========================================================================
    /// Full admin access to all secrets engines.
    /// Includes creating/deleting mounts, configuring engines, etc.
    SecretsAdmin,
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
            (Capability::Full { prefix }, Operation::Watch { key_prefix }) => key_prefix.starts_with(prefix),
            // Specific capabilities
            (Capability::Read { prefix }, Operation::Read { key }) => key.starts_with(prefix),
            (Capability::Write { prefix }, Operation::Write { key, .. }) => key.starts_with(prefix),
            (Capability::Delete { prefix }, Operation::Delete { key }) => key.starts_with(prefix),
            (Capability::Watch { prefix }, Operation::Watch { key_prefix }) => key_prefix.starts_with(prefix),
            // ClusterAdmin operations
            (Capability::ClusterAdmin, Operation::ClusterAdmin { .. }) => true,
            // ShellExecute operations
            (
                Capability::ShellExecute {
                    command_pattern,
                    working_dir: cap_wd,
                },
                Operation::ShellExecute {
                    command,
                    working_dir: op_wd,
                },
            ) => {
                // Check command pattern match
                let cmd_match = match command_pattern.as_str() {
                    "*" => true,
                    pattern if pattern.contains('*') => glob_match(pattern, command),
                    exact => exact == command,
                };
                // Check working directory constraint
                let wd_match = match (cap_wd, op_wd) {
                    (None, _) => true, // No constraint
                    (Some(cap_dir), Some(req_dir)) => req_dir.starts_with(cap_dir),
                    (Some(_), None) => false, // Capability requires wd, none provided
                };
                cmd_match && wd_match
            }

            // ==========================================================================
            // Secrets Engine authorization
            // ==========================================================================

            // SecretsAdmin authorizes all secrets operations
            (Capability::SecretsAdmin, Operation::SecretsRead { .. }) => true,
            (Capability::SecretsAdmin, Operation::SecretsWrite { .. }) => true,
            (Capability::SecretsAdmin, Operation::SecretsDelete { .. }) => true,
            (Capability::SecretsAdmin, Operation::SecretsList { .. }) => true,
            (Capability::SecretsAdmin, Operation::TransitEncrypt { .. }) => true,
            (Capability::SecretsAdmin, Operation::TransitDecrypt { .. }) => true,
            (Capability::SecretsAdmin, Operation::TransitSign { .. }) => true,
            (Capability::SecretsAdmin, Operation::TransitVerify { .. }) => true,
            (Capability::SecretsAdmin, Operation::TransitKeyManage { .. }) => true,
            (Capability::SecretsAdmin, Operation::PkiIssue { .. }) => true,
            (Capability::SecretsAdmin, Operation::PkiRevoke) => true,
            (Capability::SecretsAdmin, Operation::PkiReadCa) => true,
            (Capability::SecretsAdmin, Operation::PkiManage) => true,
            (Capability::SecretsAdmin, Operation::SecretsAdmin { .. }) => true,

            // SecretsFull authorizes all secrets KV operations for matching mount/prefix
            (
                Capability::SecretsFull {
                    mount: cap_mount,
                    prefix: cap_prefix,
                },
                Operation::SecretsRead { mount: op_mount, path },
            ) => cap_mount == op_mount && path.starts_with(cap_prefix),
            (
                Capability::SecretsFull {
                    mount: cap_mount,
                    prefix: cap_prefix,
                },
                Operation::SecretsWrite { mount: op_mount, path },
            ) => cap_mount == op_mount && path.starts_with(cap_prefix),
            (
                Capability::SecretsFull {
                    mount: cap_mount,
                    prefix: cap_prefix,
                },
                Operation::SecretsDelete { mount: op_mount, path },
            ) => cap_mount == op_mount && path.starts_with(cap_prefix),
            (
                Capability::SecretsFull {
                    mount: cap_mount,
                    prefix: cap_prefix,
                },
                Operation::SecretsList { mount: op_mount, path },
            ) => cap_mount == op_mount && path.starts_with(cap_prefix),

            // Specific secrets capabilities
            (
                Capability::SecretsRead {
                    mount: cap_mount,
                    prefix: cap_prefix,
                },
                Operation::SecretsRead { mount: op_mount, path },
            ) => cap_mount == op_mount && path.starts_with(cap_prefix),
            (
                Capability::SecretsWrite {
                    mount: cap_mount,
                    prefix: cap_prefix,
                },
                Operation::SecretsWrite { mount: op_mount, path },
            ) => cap_mount == op_mount && path.starts_with(cap_prefix),
            (
                Capability::SecretsDelete {
                    mount: cap_mount,
                    prefix: cap_prefix,
                },
                Operation::SecretsDelete { mount: op_mount, path },
            ) => cap_mount == op_mount && path.starts_with(cap_prefix),
            (
                Capability::SecretsList {
                    mount: cap_mount,
                    prefix: cap_prefix,
                },
                Operation::SecretsList { mount: op_mount, path },
            ) => cap_mount == op_mount && path.starts_with(cap_prefix),

            // ==========================================================================
            // Transit Engine authorization
            // ==========================================================================
            (Capability::TransitEncrypt { key_prefix }, Operation::TransitEncrypt { key_name }) => {
                key_name.starts_with(key_prefix)
            }
            (Capability::TransitDecrypt { key_prefix }, Operation::TransitDecrypt { key_name }) => {
                key_name.starts_with(key_prefix)
            }
            (Capability::TransitSign { key_prefix }, Operation::TransitSign { key_name }) => {
                key_name.starts_with(key_prefix)
            }
            (Capability::TransitVerify { key_prefix }, Operation::TransitVerify { key_name }) => {
                key_name.starts_with(key_prefix)
            }
            (Capability::TransitKeyManage { key_prefix }, Operation::TransitKeyManage { key_name }) => {
                key_name.starts_with(key_prefix)
            }

            // ==========================================================================
            // PKI Engine authorization
            // ==========================================================================
            (Capability::PkiIssue { role_prefix }, Operation::PkiIssue { role }) => role.starts_with(role_prefix),
            (Capability::PkiRevoke, Operation::PkiRevoke) => true,
            (Capability::PkiReadCa, Operation::PkiReadCa) => true,
            (Capability::PkiManage, Operation::PkiManage) => true,
            // PkiManage also authorizes all other PKI operations
            (Capability::PkiManage, Operation::PkiIssue { .. }) => true,
            (Capability::PkiManage, Operation::PkiRevoke) => true,
            (Capability::PkiManage, Operation::PkiReadCa) => true,

            // No match
            _ => false,
        }
    }

    /// Check if this capability authorizes executing a specific shell command.
    ///
    /// This is a convenience method for shell command authorization.
    pub fn authorizes_shell_command(&self, command: &str, working_dir: Option<&str>) -> bool {
        self.authorizes(&Operation::ShellExecute {
            command: command.to_string(),
            working_dir: working_dir.map(|s| s.to_string()),
        })
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
            (Capability::Full { prefix: p1 }, Capability::Read { prefix: p2 }) => p2.starts_with(p1),
            (Capability::Full { prefix: p1 }, Capability::Write { prefix: p2 }) => p2.starts_with(p1),
            (Capability::Full { prefix: p1 }, Capability::Delete { prefix: p2 }) => p2.starts_with(p1),
            (Capability::Full { prefix: p1 }, Capability::Watch { prefix: p2 }) => p2.starts_with(p1),
            (Capability::Full { prefix: p1 }, Capability::Full { prefix: p2 }) => p2.starts_with(p1),
            // Same type with narrower prefix
            (Capability::Read { prefix: p1 }, Capability::Read { prefix: p2 }) => p2.starts_with(p1),
            (Capability::Write { prefix: p1 }, Capability::Write { prefix: p2 }) => p2.starts_with(p1),
            (Capability::Delete { prefix: p1 }, Capability::Delete { prefix: p2 }) => p2.starts_with(p1),
            (Capability::Watch { prefix: p1 }, Capability::Watch { prefix: p2 }) => p2.starts_with(p1),
            // ClusterAdmin only contains itself
            (Capability::ClusterAdmin, Capability::ClusterAdmin) => true,
            // Delegate only contains itself
            (Capability::Delegate, Capability::Delegate) => true,
            // ShellExecute: child pattern must be subset of parent pattern
            (
                Capability::ShellExecute {
                    command_pattern: p1,
                    working_dir: wd1,
                },
                Capability::ShellExecute {
                    command_pattern: p2,
                    working_dir: wd2,
                },
            ) => {
                // Child pattern must be more specific than parent
                let pattern_ok = if p1 == "*" {
                    true // Parent allows everything
                } else if p2 == "*" {
                    false // Child wants everything but parent doesn't allow it
                } else if p1.ends_with('*') && p2.ends_with('*') {
                    // Both are prefix patterns, child must be more specific
                    p2.starts_with(p1.trim_end_matches('*'))
                } else if p1.ends_with('*') {
                    // Parent is prefix pattern, child is exact
                    p2.starts_with(p1.trim_end_matches('*'))
                } else {
                    // Parent is exact, child must be same
                    p1 == p2
                };

                // Child working_dir must be within parent's constraint
                let wd_ok = match (wd1, wd2) {
                    (None, _) => true,        // Parent has no constraint
                    (Some(_), None) => false, // Parent has constraint, child doesn't
                    (Some(parent_wd), Some(child_wd)) => child_wd.starts_with(parent_wd),
                };

                pattern_ok && wd_ok
            }

            // ==========================================================================
            // Secrets Engine capability containment
            // ==========================================================================

            // SecretsAdmin contains all secrets-related capabilities
            (Capability::SecretsAdmin, Capability::SecretsAdmin) => true,
            (Capability::SecretsAdmin, Capability::SecretsFull { .. }) => true,
            (Capability::SecretsAdmin, Capability::SecretsRead { .. }) => true,
            (Capability::SecretsAdmin, Capability::SecretsWrite { .. }) => true,
            (Capability::SecretsAdmin, Capability::SecretsDelete { .. }) => true,
            (Capability::SecretsAdmin, Capability::SecretsList { .. }) => true,
            (Capability::SecretsAdmin, Capability::TransitEncrypt { .. }) => true,
            (Capability::SecretsAdmin, Capability::TransitDecrypt { .. }) => true,
            (Capability::SecretsAdmin, Capability::TransitSign { .. }) => true,
            (Capability::SecretsAdmin, Capability::TransitVerify { .. }) => true,
            (Capability::SecretsAdmin, Capability::TransitKeyManage { .. }) => true,
            (Capability::SecretsAdmin, Capability::PkiIssue { .. }) => true,
            (Capability::SecretsAdmin, Capability::PkiRevoke) => true,
            (Capability::SecretsAdmin, Capability::PkiReadCa) => true,
            (Capability::SecretsAdmin, Capability::PkiManage) => true,

            // SecretsFull contains all secrets KV capabilities for same or narrower scope
            (Capability::SecretsFull { mount: m1, prefix: p1 }, Capability::SecretsFull { mount: m2, prefix: p2 }) => {
                m1 == m2 && p2.starts_with(p1)
            }
            (Capability::SecretsFull { mount: m1, prefix: p1 }, Capability::SecretsRead { mount: m2, prefix: p2 }) => {
                m1 == m2 && p2.starts_with(p1)
            }
            (Capability::SecretsFull { mount: m1, prefix: p1 }, Capability::SecretsWrite { mount: m2, prefix: p2 }) => {
                m1 == m2 && p2.starts_with(p1)
            }
            (
                Capability::SecretsFull { mount: m1, prefix: p1 },
                Capability::SecretsDelete { mount: m2, prefix: p2 },
            ) => m1 == m2 && p2.starts_with(p1),
            (Capability::SecretsFull { mount: m1, prefix: p1 }, Capability::SecretsList { mount: m2, prefix: p2 }) => {
                m1 == m2 && p2.starts_with(p1)
            }

            // Same type with narrower scope
            (Capability::SecretsRead { mount: m1, prefix: p1 }, Capability::SecretsRead { mount: m2, prefix: p2 }) => {
                m1 == m2 && p2.starts_with(p1)
            }
            (
                Capability::SecretsWrite { mount: m1, prefix: p1 },
                Capability::SecretsWrite { mount: m2, prefix: p2 },
            ) => m1 == m2 && p2.starts_with(p1),
            (
                Capability::SecretsDelete { mount: m1, prefix: p1 },
                Capability::SecretsDelete { mount: m2, prefix: p2 },
            ) => m1 == m2 && p2.starts_with(p1),
            (Capability::SecretsList { mount: m1, prefix: p1 }, Capability::SecretsList { mount: m2, prefix: p2 }) => {
                m1 == m2 && p2.starts_with(p1)
            }

            // Transit capabilities with prefix containment
            (Capability::TransitEncrypt { key_prefix: p1 }, Capability::TransitEncrypt { key_prefix: p2 }) => {
                p2.starts_with(p1)
            }
            (Capability::TransitDecrypt { key_prefix: p1 }, Capability::TransitDecrypt { key_prefix: p2 }) => {
                p2.starts_with(p1)
            }
            (Capability::TransitSign { key_prefix: p1 }, Capability::TransitSign { key_prefix: p2 }) => {
                p2.starts_with(p1)
            }
            (Capability::TransitVerify { key_prefix: p1 }, Capability::TransitVerify { key_prefix: p2 }) => {
                p2.starts_with(p1)
            }
            (Capability::TransitKeyManage { key_prefix: p1 }, Capability::TransitKeyManage { key_prefix: p2 }) => {
                p2.starts_with(p1)
            }

            // PKI capabilities
            (Capability::PkiManage, Capability::PkiManage) => true,
            (Capability::PkiManage, Capability::PkiIssue { .. }) => true,
            (Capability::PkiManage, Capability::PkiRevoke) => true,
            (Capability::PkiManage, Capability::PkiReadCa) => true,
            (Capability::PkiIssue { role_prefix: p1 }, Capability::PkiIssue { role_prefix: p2 }) => p2.starts_with(p1),
            (Capability::PkiRevoke, Capability::PkiRevoke) => true,
            (Capability::PkiReadCa, Capability::PkiReadCa) => true,

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
    /// Shell command execution.
    ShellExecute {
        /// Command to execute.
        command: String,
        /// Working directory for execution.
        working_dir: Option<String>,
    },

    // ==========================================================================
    // Secrets Engine Operations
    // ==========================================================================
    /// Read a secret from a mount.
    SecretsRead {
        /// Mount path.
        mount: String,
        /// Secret path within the mount.
        path: String,
    },
    /// Write a secret to a mount.
    SecretsWrite {
        /// Mount path.
        mount: String,
        /// Secret path within the mount.
        path: String,
    },
    /// Delete a secret from a mount.
    SecretsDelete {
        /// Mount path.
        mount: String,
        /// Secret path within the mount.
        path: String,
    },
    /// List secrets at a mount path.
    SecretsList {
        /// Mount path.
        mount: String,
        /// Path prefix to list.
        path: String,
    },

    // ==========================================================================
    // Transit Engine Operations
    // ==========================================================================
    /// Encrypt data with a transit key.
    TransitEncrypt {
        /// Key name.
        key_name: String,
    },
    /// Decrypt data with a transit key.
    TransitDecrypt {
        /// Key name.
        key_name: String,
    },
    /// Sign data with a transit key.
    TransitSign {
        /// Key name.
        key_name: String,
    },
    /// Verify signature with a transit key.
    TransitVerify {
        /// Key name.
        key_name: String,
    },
    /// Manage a transit key (create, rotate, delete).
    TransitKeyManage {
        /// Key name.
        key_name: String,
    },

    // ==========================================================================
    // PKI Engine Operations
    // ==========================================================================
    /// Issue a certificate using a role.
    PkiIssue {
        /// Role name.
        role: String,
    },
    /// Revoke a certificate.
    PkiRevoke,
    /// Read CA certificate.
    PkiReadCa,
    /// Manage PKI configuration.
    PkiManage,

    // ==========================================================================
    // Secrets Admin Operations
    // ==========================================================================
    /// Administrative operation on secrets engines.
    SecretsAdmin {
        /// Description of admin action.
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
            Operation::ShellExecute { command, working_dir } => {
                write!(f, "ShellExecute({}, wd={:?})", command, working_dir.as_deref().unwrap_or("<default>"))
            }
            // Secrets operations
            Operation::SecretsRead { mount, path } => write!(f, "SecretsRead({}/{})", mount, path),
            Operation::SecretsWrite { mount, path } => write!(f, "SecretsWrite({}/{})", mount, path),
            Operation::SecretsDelete { mount, path } => write!(f, "SecretsDelete({}/{})", mount, path),
            Operation::SecretsList { mount, path } => write!(f, "SecretsList({}/{})", mount, path),
            // Transit operations
            Operation::TransitEncrypt { key_name } => write!(f, "TransitEncrypt({})", key_name),
            Operation::TransitDecrypt { key_name } => write!(f, "TransitDecrypt({})", key_name),
            Operation::TransitSign { key_name } => write!(f, "TransitSign({})", key_name),
            Operation::TransitVerify { key_name } => write!(f, "TransitVerify({})", key_name),
            Operation::TransitKeyManage { key_name } => write!(f, "TransitKeyManage({})", key_name),
            // PKI operations
            Operation::PkiIssue { role } => write!(f, "PkiIssue({})", role),
            Operation::PkiRevoke => write!(f, "PkiRevoke"),
            Operation::PkiReadCa => write!(f, "PkiReadCa"),
            Operation::PkiManage => write!(f, "PkiManage"),
            // Secrets admin
            Operation::SecretsAdmin { action } => write!(f, "SecretsAdmin({})", action),
        }
    }
}
