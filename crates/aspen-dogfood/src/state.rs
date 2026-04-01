//! Persistent state file for cross-invocation cluster tracking.

use serde::Deserialize;
use serde::Serialize;
use snafu::ResultExt;

use crate::error::DogfoodResult;
use crate::error::NoClusterSnafu;
use crate::error::StateDeserializeSnafu;
use crate::error::StateFileSnafu;
use crate::error::StateSerializeSnafu;

/// Persisted state written by `start`, read by all other subcommands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DogfoodState {
    /// Node entries (one per cluster node).
    pub nodes: Vec<NodeEntry>,
    /// Whether this is a federation deployment.
    pub is_federation: bool,
    /// Whether VM-CI mode is active.
    pub vm_ci: bool,
}

/// A single managed node's identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEntry {
    /// OS process ID of the aspen-node.
    pub pid: u32,
    /// Cluster ticket string for connecting via `AspenClient`.
    pub ticket: String,
    /// Iroh endpoint address (hex-encoded PublicKey).
    pub endpoint_addr: String,
    /// Human label ("node1", "alice", "bob").
    pub label: String,
}

impl DogfoodState {
    /// Create state for a single-node cluster.
    pub fn new_single(pid: u32, ticket: String, endpoint_addr: String, vm_ci: bool) -> Self {
        Self {
            nodes: vec![NodeEntry {
                pid,
                ticket,
                endpoint_addr,
                label: "node1".to_string(),
            }],
            is_federation: false,
            vm_ci,
        }
    }

    /// Create state for a two-cluster federation.
    pub fn new_federation(
        alice_pid: u32,
        alice_ticket: String,
        alice_addr: String,
        bob_pid: u32,
        bob_ticket: String,
        bob_addr: String,
        vm_ci: bool,
    ) -> Self {
        Self {
            nodes: vec![
                NodeEntry {
                    pid: alice_pid,
                    ticket: alice_ticket,
                    endpoint_addr: alice_addr,
                    label: "alice".to_string(),
                },
                NodeEntry {
                    pid: bob_pid,
                    ticket: bob_ticket,
                    endpoint_addr: bob_addr,
                    label: "bob".to_string(),
                },
            ],
            is_federation: false,
            vm_ci,
        }
    }

    /// All node PIDs.
    pub fn node_pids(&self) -> Vec<u32> {
        self.nodes.iter().map(|n| n.pid).collect()
    }

    /// All cluster tickets.
    pub fn tickets(&self) -> Vec<String> {
        self.nodes.iter().map(|n| n.ticket.clone()).collect()
    }

    /// Primary cluster ticket (first node / alice).
    pub fn primary_ticket(&self) -> &str {
        &self.nodes[0].ticket
    }
}

// ── File operations ──────────────────────────────────────────────────

/// Write the state file as pretty-printed JSON.
pub fn write_state(path: &str, state: &DogfoodState) -> DogfoodResult<()> {
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent).context(StateFileSnafu {
            operation: "creating directory for",
            path,
        })?;
    }

    let json = serde_json::to_string_pretty(state).context(StateSerializeSnafu)?;
    std::fs::write(path, json).context(StateFileSnafu {
        operation: "writing",
        path,
    })
}

/// Read the state file, returning `NoCluster` if missing.
pub fn read_state(path: &str) -> DogfoodResult<DogfoodState> {
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return NoClusterSnafu.fail();
        }
        Err(e) => {
            return Err(e).context(StateFileSnafu {
                operation: "reading",
                path,
            });
        }
    };
    serde_json::from_str(&data).context(StateDeserializeSnafu { path })
}

/// Delete the state file (ignores "not found").
pub fn delete_state(path: &str) -> DogfoodResult<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).context(StateFileSnafu {
            operation: "deleting",
            path,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_roundtrip_single() {
        let state = DogfoodState::new_single(1234, "ticket-abc".to_string(), "addr-xyz".to_string(), false);
        let json = serde_json::to_string_pretty(&state).unwrap();
        let restored: DogfoodState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.nodes.len(), 1);
        assert_eq!(restored.nodes[0].pid, 1234);
        assert_eq!(restored.nodes[0].ticket, "ticket-abc");
        assert!(!restored.is_federation);
        assert!(!restored.vm_ci);
    }

    #[test]
    fn state_roundtrip_federation() {
        let state = DogfoodState::new_federation(
            100,
            "t-alice".to_string(),
            "a-alice".to_string(),
            200,
            "t-bob".to_string(),
            "a-bob".to_string(),
            true,
        );
        let json = serde_json::to_string_pretty(&state).unwrap();
        let restored: DogfoodState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.nodes.len(), 2);
        assert_eq!(restored.nodes[0].label, "alice");
        assert_eq!(restored.nodes[1].label, "bob");
        assert!(restored.vm_ci);
    }

    #[test]
    fn node_pids_and_tickets() {
        let state = DogfoodState::new_federation(10, "t1".into(), "a1".into(), 20, "t2".into(), "a2".into(), false);
        assert_eq!(state.node_pids(), vec![10, 20]);
        assert_eq!(state.tickets(), vec!["t1", "t2"]);
        assert_eq!(state.primary_ticket(), "t1");
    }

    #[test]
    fn state_file_write_read_delete() {
        let dir = tempfile::tempdir().unwrap();
        let path = format!("{}/dogfood-state.json", dir.path().display());

        let state = DogfoodState::new_single(42, "tk".into(), "ep".into(), true);
        write_state(&path, &state).unwrap();

        let restored = read_state(&path).unwrap();
        assert_eq!(restored.nodes[0].pid, 42);
        assert!(restored.vm_ci);

        delete_state(&path).unwrap();
        assert!(matches!(read_state(&path), Err(crate::error::DogfoodError::NoCluster)));
    }

    #[test]
    fn read_state_missing_returns_no_cluster() {
        let result = read_state("/tmp/nonexistent-dogfood-state-12345.json");
        assert!(matches!(result, Err(crate::error::DogfoodError::NoCluster)));
    }

    #[test]
    fn read_state_corrupt_returns_deserialize_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = format!("{}/dogfood-state.json", dir.path().display());
        std::fs::write(&path, "not json").unwrap();

        let result = read_state(&path);
        assert!(matches!(result, Err(crate::error::DogfoodError::StateDeserialize { .. })));
    }

    #[test]
    fn delete_state_nonexistent_is_ok() {
        let result = delete_state("/tmp/nonexistent-dogfood-state-12345.json");
        assert!(result.is_ok());
    }
}
