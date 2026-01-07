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
        }
    }
}
