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

    // ==========================================================================
    // Net Service Mesh Capabilities
    // ==========================================================================
    /// Connect to named services through the mesh.
    NetConnect {
        /// Service name prefix (e.g., "prod/" matches "prod/mydb").
        service_prefix: String,
    },
    /// Publish/unpublish services in the registry.
    NetPublish {
        /// Service name prefix.
        service_prefix: String,
    },
    /// Full net admin access (registry, DNS overrides, all net operations).
    NetAdmin,

    // ==========================================================================
    // Federation Sync Capabilities
    // ==========================================================================
    /// Pull (read) federated resources matching a repo prefix.
    ///
    /// Authorizes SyncObjects / GetResourceState / pull operations for
    /// resources whose federated ID starts with `repo_prefix`.
    /// An empty prefix matches all repos.
    FederationPull {
        /// Federated resource ID prefix (e.g., "forge:" or "forge:org-a/").
        repo_prefix: String,
    },
    /// Push (write) federated resources matching a repo prefix.
    ///
    /// Authorizes PushObjects / ref updates for resources whose federated
    /// ID starts with `repo_prefix`. An empty prefix matches all repos.
    FederationPush {
        /// Federated resource ID prefix.
        repo_prefix: String,
    },
}

fn matches_prefix_scope(prefix: &str, value: &str) -> bool {
    value.starts_with(prefix)
}

fn matches_mount_scope(cap_mount: &str, op_mount: &str, cap_prefix: &str, path: &str) -> bool {
    cap_mount == op_mount && matches_prefix_scope(cap_prefix, path)
}

fn shell_command_matches(pattern: &str, command: &str) -> bool {
    match pattern {
        "*" => true,
        pattern if pattern.contains('*') => glob_match(pattern, command),
        exact => exact == command,
    }
}

fn shell_working_dir_matches(capability_dir: &Option<String>, requested_dir: &Option<String>) -> bool {
    match (capability_dir, requested_dir) {
        (None, _) => true,
        (Some(parent), Some(child)) => child.starts_with(parent),
        (Some(_), None) => false,
    }
}

fn shell_pattern_contains(parent_pattern: &str, child_pattern: &str) -> bool {
    if parent_pattern == "*" {
        return true;
    }
    if child_pattern == "*" {
        return false;
    }
    if parent_pattern.ends_with('*') && child_pattern.ends_with('*') {
        return child_pattern.starts_with(parent_pattern.trim_end_matches('*'));
    }
    if parent_pattern.ends_with('*') {
        return child_pattern.starts_with(parent_pattern.trim_end_matches('*'));
    }
    parent_pattern == child_pattern
}

impl Capability {
    /// Check if this capability authorizes the given operation.
    ///
    /// Returns true if this capability grants permission for the operation.
    pub fn authorizes(&self, op: &Operation) -> bool {
        self.authorizes_data(op)
            .or_else(|| self.authorizes_shell(op))
            .or_else(|| self.authorizes_secrets(op))
            .or_else(|| self.authorizes_transit(op))
            .or_else(|| self.authorizes_pki(op))
            .or_else(|| self.authorizes_net(op))
            .or_else(|| self.authorizes_federation(op))
            .unwrap_or(false)
    }

    fn authorizes_data(&self, op: &Operation) -> Option<bool> {
        match (self, op) {
            (Capability::Full { prefix }, Operation::Read { key }) => Some(matches_prefix_scope(prefix, key)),
            (Capability::Full { prefix }, Operation::Write { key, .. }) => Some(matches_prefix_scope(prefix, key)),
            (Capability::Full { prefix }, Operation::Delete { key }) => Some(matches_prefix_scope(prefix, key)),
            (Capability::Full { prefix }, Operation::Watch { key_prefix }) => {
                Some(matches_prefix_scope(prefix, key_prefix))
            }
            (Capability::Read { prefix }, Operation::Read { key }) => Some(matches_prefix_scope(prefix, key)),
            (Capability::Write { prefix }, Operation::Write { key, .. }) => Some(matches_prefix_scope(prefix, key)),
            (Capability::Delete { prefix }, Operation::Delete { key }) => Some(matches_prefix_scope(prefix, key)),
            (Capability::Watch { prefix }, Operation::Watch { key_prefix }) => {
                Some(matches_prefix_scope(prefix, key_prefix))
            }
            (Capability::ClusterAdmin, Operation::ClusterAdmin { .. }) => Some(true),
            _ => None,
        }
    }

    fn authorizes_shell(&self, op: &Operation) -> Option<bool> {
        match (self, op) {
            (
                Capability::ShellExecute {
                    command_pattern,
                    working_dir,
                },
                Operation::ShellExecute {
                    command,
                    working_dir: requested_dir,
                },
            ) => Some(
                shell_command_matches(command_pattern, command)
                    && shell_working_dir_matches(working_dir, requested_dir),
            ),
            _ => None,
        }
    }

    fn authorizes_secrets(&self, op: &Operation) -> Option<bool> {
        match (self, op) {
            (Capability::SecretsAdmin, Operation::SecretsRead { .. })
            | (Capability::SecretsAdmin, Operation::SecretsWrite { .. })
            | (Capability::SecretsAdmin, Operation::SecretsDelete { .. })
            | (Capability::SecretsAdmin, Operation::SecretsList { .. })
            | (Capability::SecretsAdmin, Operation::SecretsAdmin { .. }) => Some(true),
            (
                Capability::SecretsFull { mount, prefix } | Capability::SecretsRead { mount, prefix },
                Operation::SecretsRead { mount: op_mount, path },
            ) => Some(matches_mount_scope(mount, op_mount, prefix, path)),
            (
                Capability::SecretsFull { mount, prefix } | Capability::SecretsWrite { mount, prefix },
                Operation::SecretsWrite { mount: op_mount, path },
            ) => Some(matches_mount_scope(mount, op_mount, prefix, path)),
            (
                Capability::SecretsFull { mount, prefix } | Capability::SecretsDelete { mount, prefix },
                Operation::SecretsDelete { mount: op_mount, path },
            ) => Some(matches_mount_scope(mount, op_mount, prefix, path)),
            (
                Capability::SecretsFull { mount, prefix } | Capability::SecretsList { mount, prefix },
                Operation::SecretsList { mount: op_mount, path },
            ) => Some(matches_mount_scope(mount, op_mount, prefix, path)),
            _ => None,
        }
    }

    fn authorizes_transit(&self, op: &Operation) -> Option<bool> {
        match (self, op) {
            (Capability::SecretsAdmin, Operation::TransitEncrypt { .. })
            | (Capability::SecretsAdmin, Operation::TransitDecrypt { .. })
            | (Capability::SecretsAdmin, Operation::TransitSign { .. })
            | (Capability::SecretsAdmin, Operation::TransitVerify { .. })
            | (Capability::SecretsAdmin, Operation::TransitKeyManage { .. }) => Some(true),
            (Capability::TransitEncrypt { key_prefix }, Operation::TransitEncrypt { key_name })
            | (Capability::TransitDecrypt { key_prefix }, Operation::TransitDecrypt { key_name })
            | (Capability::TransitSign { key_prefix }, Operation::TransitSign { key_name })
            | (Capability::TransitVerify { key_prefix }, Operation::TransitVerify { key_name })
            | (Capability::TransitKeyManage { key_prefix }, Operation::TransitKeyManage { key_name }) => {
                Some(matches_prefix_scope(key_prefix, key_name))
            }
            _ => None,
        }
    }

    fn authorizes_pki(&self, op: &Operation) -> Option<bool> {
        match (self, op) {
            (Capability::SecretsAdmin, Operation::PkiIssue { .. })
            | (Capability::SecretsAdmin, Operation::PkiRevoke)
            | (Capability::SecretsAdmin, Operation::PkiReadCa)
            | (Capability::SecretsAdmin, Operation::PkiManage) => Some(true),
            (Capability::PkiIssue { role_prefix }, Operation::PkiIssue { role }) => {
                Some(matches_prefix_scope(role_prefix, role))
            }
            (Capability::PkiRevoke, Operation::PkiRevoke)
            | (Capability::PkiReadCa, Operation::PkiReadCa)
            | (Capability::PkiManage, Operation::PkiManage)
            | (Capability::PkiManage, Operation::PkiRevoke)
            | (Capability::PkiManage, Operation::PkiReadCa)
            | (Capability::PkiManage, Operation::PkiIssue { .. }) => Some(true),
            _ => None,
        }
    }

    fn authorizes_net(&self, op: &Operation) -> Option<bool> {
        match (self, op) {
            (Capability::NetAdmin, Operation::NetConnect { .. })
            | (Capability::NetAdmin, Operation::NetPublish { .. })
            | (Capability::NetAdmin, Operation::NetUnpublish { .. })
            | (Capability::NetAdmin, Operation::NetAdmin { .. }) => Some(true),
            (Capability::NetConnect { service_prefix }, Operation::NetConnect { service, .. }) => {
                Some(matches_prefix_scope(service_prefix, service))
            }
            (Capability::NetPublish { service_prefix }, Operation::NetPublish { service })
            | (Capability::NetPublish { service_prefix }, Operation::NetUnpublish { service }) => {
                Some(matches_prefix_scope(service_prefix, service))
            }
            _ => None,
        }
    }

    fn authorizes_federation(&self, op: &Operation) -> Option<bool> {
        match (self, op) {
            (Capability::FederationPull { repo_prefix }, Operation::FederationPull { fed_id })
            | (Capability::FederationPush { repo_prefix }, Operation::FederationPush { fed_id }) => {
                Some(matches_prefix_scope(repo_prefix, fed_id))
            }
            _ => None,
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
        self.contains_data(other)
            .or_else(|| self.contains_shell(other))
            .or_else(|| self.contains_secrets(other))
            .or_else(|| self.contains_transit(other))
            .or_else(|| self.contains_pki(other))
            .or_else(|| self.contains_net(other))
            .or_else(|| self.contains_federation(other))
            .unwrap_or(false)
    }

    fn contains_data(&self, other: &Capability) -> Option<bool> {
        match (self, other) {
            (Capability::Full { prefix: parent }, Capability::Read { prefix: child })
            | (Capability::Full { prefix: parent }, Capability::Write { prefix: child })
            | (Capability::Full { prefix: parent }, Capability::Delete { prefix: child })
            | (Capability::Full { prefix: parent }, Capability::Watch { prefix: child })
            | (Capability::Full { prefix: parent }, Capability::Full { prefix: child })
            | (Capability::Read { prefix: parent }, Capability::Read { prefix: child })
            | (Capability::Write { prefix: parent }, Capability::Write { prefix: child })
            | (Capability::Delete { prefix: parent }, Capability::Delete { prefix: child })
            | (Capability::Watch { prefix: parent }, Capability::Watch { prefix: child }) => {
                Some(matches_prefix_scope(parent, child))
            }
            (Capability::ClusterAdmin, Capability::ClusterAdmin) | (Capability::Delegate, Capability::Delegate) => {
                Some(true)
            }
            _ => None,
        }
    }

    fn contains_shell(&self, other: &Capability) -> Option<bool> {
        match (self, other) {
            (
                Capability::ShellExecute {
                    command_pattern: parent_pattern,
                    working_dir: parent_dir,
                },
                Capability::ShellExecute {
                    command_pattern: child_pattern,
                    working_dir: child_dir,
                },
            ) => Some(
                shell_pattern_contains(parent_pattern, child_pattern)
                    && shell_working_dir_matches(parent_dir, child_dir),
            ),
            _ => None,
        }
    }

    fn contains_secrets(&self, other: &Capability) -> Option<bool> {
        self.contains_secrets_admin(other)
            .or_else(|| self.contains_secrets_full(other))
            .or_else(|| self.contains_secrets_scoped(other))
    }

    fn contains_secrets_admin(&self, other: &Capability) -> Option<bool> {
        match (self, other) {
            (Capability::SecretsAdmin, Capability::SecretsAdmin)
            | (Capability::SecretsAdmin, Capability::SecretsFull { .. })
            | (Capability::SecretsAdmin, Capability::SecretsRead { .. })
            | (Capability::SecretsAdmin, Capability::SecretsWrite { .. })
            | (Capability::SecretsAdmin, Capability::SecretsDelete { .. })
            | (Capability::SecretsAdmin, Capability::SecretsList { .. }) => Some(true),
            _ => None,
        }
    }

    fn contains_secrets_full(&self, other: &Capability) -> Option<bool> {
        match (self, other) {
            (
                Capability::SecretsFull {
                    mount: parent_mount,
                    prefix: parent_prefix,
                },
                Capability::SecretsFull {
                    mount: child_mount,
                    prefix: child_prefix,
                }
                | Capability::SecretsRead {
                    mount: child_mount,
                    prefix: child_prefix,
                }
                | Capability::SecretsWrite {
                    mount: child_mount,
                    prefix: child_prefix,
                }
                | Capability::SecretsDelete {
                    mount: child_mount,
                    prefix: child_prefix,
                }
                | Capability::SecretsList {
                    mount: child_mount,
                    prefix: child_prefix,
                },
            ) => Some(matches_mount_scope(parent_mount, child_mount, parent_prefix, child_prefix)),
            _ => None,
        }
    }

    fn contains_secrets_scoped(&self, other: &Capability) -> Option<bool> {
        match (self, other) {
            (
                Capability::SecretsRead {
                    mount: parent_mount,
                    prefix: parent_prefix,
                },
                Capability::SecretsRead {
                    mount: child_mount,
                    prefix: child_prefix,
                },
            )
            | (
                Capability::SecretsWrite {
                    mount: parent_mount,
                    prefix: parent_prefix,
                },
                Capability::SecretsWrite {
                    mount: child_mount,
                    prefix: child_prefix,
                },
            )
            | (
                Capability::SecretsDelete {
                    mount: parent_mount,
                    prefix: parent_prefix,
                },
                Capability::SecretsDelete {
                    mount: child_mount,
                    prefix: child_prefix,
                },
            )
            | (
                Capability::SecretsList {
                    mount: parent_mount,
                    prefix: parent_prefix,
                },
                Capability::SecretsList {
                    mount: child_mount,
                    prefix: child_prefix,
                },
            ) => Some(matches_mount_scope(parent_mount, child_mount, parent_prefix, child_prefix)),
            _ => None,
        }
    }

    fn contains_transit(&self, other: &Capability) -> Option<bool> {
        match (self, other) {
            (Capability::SecretsAdmin, Capability::TransitEncrypt { .. })
            | (Capability::SecretsAdmin, Capability::TransitDecrypt { .. })
            | (Capability::SecretsAdmin, Capability::TransitSign { .. })
            | (Capability::SecretsAdmin, Capability::TransitVerify { .. })
            | (Capability::SecretsAdmin, Capability::TransitKeyManage { .. }) => Some(true),
            (Capability::TransitEncrypt { key_prefix: parent }, Capability::TransitEncrypt { key_prefix: child })
            | (Capability::TransitDecrypt { key_prefix: parent }, Capability::TransitDecrypt { key_prefix: child })
            | (Capability::TransitSign { key_prefix: parent }, Capability::TransitSign { key_prefix: child })
            | (Capability::TransitVerify { key_prefix: parent }, Capability::TransitVerify { key_prefix: child })
            | (
                Capability::TransitKeyManage { key_prefix: parent },
                Capability::TransitKeyManage { key_prefix: child },
            ) => Some(matches_prefix_scope(parent, child)),
            _ => None,
        }
    }

    fn contains_pki(&self, other: &Capability) -> Option<bool> {
        match (self, other) {
            (Capability::SecretsAdmin, Capability::PkiIssue { .. })
            | (Capability::SecretsAdmin, Capability::PkiRevoke)
            | (Capability::SecretsAdmin, Capability::PkiReadCa)
            | (Capability::SecretsAdmin, Capability::PkiManage) => Some(true),
            (Capability::PkiManage, Capability::PkiManage)
            | (Capability::PkiManage, Capability::PkiIssue { .. })
            | (Capability::PkiManage, Capability::PkiRevoke)
            | (Capability::PkiManage, Capability::PkiReadCa)
            | (Capability::PkiRevoke, Capability::PkiRevoke)
            | (Capability::PkiReadCa, Capability::PkiReadCa) => Some(true),
            (Capability::PkiIssue { role_prefix: parent }, Capability::PkiIssue { role_prefix: child }) => {
                Some(matches_prefix_scope(parent, child))
            }
            _ => None,
        }
    }

    fn contains_net(&self, other: &Capability) -> Option<bool> {
        match (self, other) {
            (Capability::NetAdmin, Capability::NetAdmin)
            | (Capability::NetAdmin, Capability::NetConnect { .. })
            | (Capability::NetAdmin, Capability::NetPublish { .. }) => Some(true),
            (Capability::NetConnect { service_prefix: parent }, Capability::NetConnect { service_prefix: child })
            | (Capability::NetPublish { service_prefix: parent }, Capability::NetPublish { service_prefix: child }) => {
                Some(matches_prefix_scope(parent, child))
            }
            _ => None,
        }
    }

    fn contains_federation(&self, other: &Capability) -> Option<bool> {
        match (self, other) {
            (Capability::FederationPull { repo_prefix: parent }, Capability::FederationPull { repo_prefix: child })
            | (Capability::FederationPush { repo_prefix: parent }, Capability::FederationPush { repo_prefix: child }) => {
                Some(matches_prefix_scope(parent, child))
            }
            _ => None,
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

    // ==========================================================================
    // Net Service Mesh Operations
    // ==========================================================================
    /// Connect to a named service through the mesh.
    NetConnect {
        /// Service name.
        service: String,
        /// Target port.
        port: u16,
    },
    /// Publish a service to the mesh registry.
    NetPublish {
        /// Service name.
        service: String,
    },
    /// Unpublish a service from the mesh registry.
    NetUnpublish {
        /// Service name.
        service: String,
    },
    /// Net admin operation.
    NetAdmin {
        /// Description of admin action.
        action: String,
    },

    // ==========================================================================
    // Federation Sync Operations
    // ==========================================================================
    /// Pull (read) a federated resource.
    FederationPull {
        /// Federated resource ID (short form).
        fed_id: String,
    },
    /// Push (write) to a federated resource.
    FederationPush {
        /// Federated resource ID (short form).
        fed_id: String,
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
            // Net operations
            Operation::NetConnect { service, port } => write!(f, "NetConnect({service}:{port})"),
            Operation::NetPublish { service } => write!(f, "NetPublish({service})"),
            Operation::NetUnpublish { service } => write!(f, "NetUnpublish({service})"),
            Operation::NetAdmin { action } => write!(f, "NetAdmin({action})"),
            // Federation operations
            Operation::FederationPull { fed_id } => write!(f, "FederationPull({fed_id})"),
            Operation::FederationPush { fed_id } => write!(f, "FederationPush({fed_id})"),
        }
    }
}
