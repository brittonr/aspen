//! Test harness for patchbay-based network simulation.
//!
//! `PatchbayHarness` manages a patchbay `Lab`, spawns Aspen nodes inside
//! device namespaces, and provides helpers for cluster initialization,
//! KV operations, and leader checks.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use aspen_cluster_types::ClusterNode;
use aspen_cluster_types::NodeAddress;
use aspen_raft::types::NodeId;
use aspen_testing_core::WaitError;
use aspen_testing_core::wait_until;
use iroh::EndpointAddr;
use patchbay::Device;
use patchbay::Lab;
use patchbay::LinkCondition;
use patchbay::Region;
use patchbay::Router;
use patchbay::RouterPreset;
use tracing::info;

use crate::node::NodeHandle;
use crate::node::{self};

/// Maximum time to wait for leader election.
const LEADER_ELECTION_TIMEOUT: Duration = Duration::from_secs(30);

/// Poll interval when waiting for leader election.
const LEADER_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Test harness wrapping a patchbay lab with Aspen nodes.
pub struct PatchbayHarness {
    /// The patchbay lab owning all namespaces.
    pub lab: Lab,
    /// Node handles indexed by NodeId.
    handles: Vec<NodeHandle>,
    /// Devices indexed by NodeId.
    devices: Vec<Device>,
    /// Routers created in this harness.
    pub(crate) routers: Vec<Router>,
    /// Named regions for multi-region topologies.
    pub(crate) regions: HashMap<String, Region>,
}

impl PatchbayHarness {
    /// Create a new harness. Caller must have already called `patchbay::init_userns()`.
    pub async fn new() -> Result<Self> {
        let lab = Lab::new().await.context("failed to create patchbay lab")?;
        Ok(Self {
            lab,
            handles: Vec::new(),
            devices: Vec::new(),
            routers: Vec::new(),
            regions: HashMap::new(),
        })
    }

    /// Add a router to the lab.
    pub async fn add_router(&mut self, name: &str, preset: RouterPreset) -> Result<usize> {
        let router = self
            .lab
            .add_router(name)
            .preset(preset)
            .build()
            .await
            .with_context(|| format!("failed to create router '{}'", name))?;
        let idx = self.routers.len();
        self.routers.push(router);
        Ok(idx)
    }

    /// Add a node (device + Aspen node) behind a router.
    ///
    /// Returns the NodeId assigned to this node.
    pub async fn add_node(
        &mut self,
        name: &str,
        router_idx: usize,
        link_condition: Option<LinkCondition>,
    ) -> Result<NodeId> {
        let router = &self.routers[router_idx];
        let device = self
            .lab
            .add_device(name)
            .iface("eth0", router.id(), link_condition)
            .build()
            .await
            .with_context(|| format!("failed to create device '{}'", name))?;

        let node_id: NodeId = (self.handles.len() as u64 + 1).into();
        let node_id_copy = node_id;

        // Spawn the node inside the device's network namespace.
        // The closure runs indefinitely (command loop), so we use a oneshot
        // channel to get the handle back before the closure completes.
        let (handle_tx, handle_rx) = tokio::sync::oneshot::channel();
        let device_clone = device.clone();

        device_clone.spawn(async move |_dev| {
            let (handle, cmd_loop) = node::bootstrap_node(node_id_copy).await?;
            // Send handle back to the test thread
            let _ = handle_tx.send(handle);
            // Run command loop until shutdown
            cmd_loop.await;
            anyhow::Ok(())
        })?;

        let handle = handle_rx.await.context("failed to receive node handle from namespace")?;

        self.handles.push(handle);
        self.devices.push(device);

        info!(node_id = node_id.0, name, "node added to harness");
        Ok(node_id)
    }

    /// Initialize the Raft cluster across all nodes.
    ///
    /// Collects addresses from all nodes, distributes peer maps, then
    /// initializes Raft on node 0 (which becomes the initial leader candidate).
    pub async fn init_cluster(&self) -> Result<()> {
        if self.handles.is_empty() {
            bail!("no nodes to initialize");
        }

        // Collect all node addresses
        let mut addrs: HashMap<NodeId, EndpointAddr> = HashMap::new();
        for handle in &self.handles {
            let addr = handle.addr().await?;
            addrs.insert(handle.node_id, addr);
        }

        // Distribute peer addresses to all nodes
        for handle in &self.handles {
            handle.add_peers(addrs.clone()).await?;
        }

        // Build ClusterNode list for Raft initialization
        let members: Vec<ClusterNode> = addrs
            .iter()
            .map(|(id, addr)| ClusterNode {
                id: id.0,
                addr: format!("node-{}", id.0),
                raft_addr: None,
                node_addr: Some(NodeAddress::new(addr.clone())),
                relay_url: None,
            })
            .collect();

        // Initialize Raft on the first node
        self.handles[0].init_cluster(members).await?;

        // Wait for leader election
        self.wait_for_leader(LEADER_ELECTION_TIMEOUT).await?;

        info!("cluster initialized with {} nodes", self.handles.len());
        Ok(())
    }

    /// Wait until all nodes agree on a leader.
    pub async fn wait_for_leader(&self, timeout: Duration) -> Result<NodeId> {
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            if tokio::time::Instant::now() > deadline {
                bail!("timed out waiting for leader election after {:?}", timeout);
            }

            if let Some(leader) = self.check_leader_consensus().await {
                return Ok(leader);
            }

            tokio::time::sleep(LEADER_POLL_INTERVAL).await;
        }
    }

    /// Check if all nodes agree on the same leader. Returns the leader if consensus.
    async fn check_leader_consensus(&self) -> Option<NodeId> {
        let mut leaders: Vec<Option<NodeId>> = Vec::new();
        for handle in &self.handles {
            match handle.get_leader().await {
                Ok(leader) => leaders.push(leader),
                Err(_) => return None,
            }
        }

        // All must agree on a non-None leader
        let first = leaders.first()?.as_ref()?;
        if leaders.iter().all(|l| l.as_ref() == Some(first)) {
            Some(*first)
        } else {
            None
        }
    }

    /// Check that all nodes agree on a leader and return it.
    pub async fn check_leader(&self) -> Result<NodeId> {
        self.check_leader_consensus().await.context("nodes do not agree on a leader")
    }

    /// Write a KV pair through the current leader.
    pub async fn write_kv(&self, key: &str, value: &str) -> Result<()> {
        let leader = self.check_leader().await?;
        let handle = self.node_handle(leader)?;
        handle.write_kv(key, value).await
    }

    /// Read a value from a specific node's state machine.
    pub async fn read_kv(&self, node_idx: usize, key: &str) -> Result<Option<String>> {
        let handle = self.handles.get(node_idx).context("node index out of bounds")?;
        handle.read_kv(key).await
    }

    /// Get a handle to a specific node by NodeId.
    pub fn node_handle(&self, node_id: NodeId) -> Result<&NodeHandle> {
        self.handles
            .iter()
            .find(|h| h.node_id == node_id)
            .with_context(|| format!("no node with id {}", node_id.0))
    }

    /// Get all node handles.
    pub fn handles(&self) -> &[NodeHandle] {
        &self.handles
    }

    /// Get all devices.
    pub fn devices(&self) -> &[Device] {
        &self.devices
    }

    /// Get all routers.
    pub fn routers(&self) -> &[Router] {
        &self.routers
    }

    /// Number of nodes in the harness.
    pub fn node_count(&self) -> usize {
        self.handles.len()
    }

    /// Get a named region.
    pub fn region(&self, name: &str) -> Option<&Region> {
        self.regions.get(name)
    }

    /// Shut down all nodes.
    pub async fn shutdown(&self) {
        for handle in &self.handles {
            let _ = handle.shutdown().await;
        }
    }
}

impl Drop for PatchbayHarness {
    fn drop(&mut self) {
        // Nodes will be cleaned up when the Lab is dropped (namespace cleanup).
        // The command loops will terminate when channels are dropped.
    }
}

#[async_trait::async_trait]
impl aspen_testing_core::TestCluster for PatchbayHarness {
    async fn wait_for_leader(&self, timeout: Duration) -> Result<u64, WaitError> {
        let handles = self.handles().to_vec();

        wait_until("patchbay leader consensus", timeout, LEADER_POLL_INTERVAL, move || {
            let hs = handles.clone();
            async move {
                let mut leaders: Vec<Option<NodeId>> = Vec::new();
                for h in &hs {
                    match h.get_leader().await {
                        Ok(l) => leaders.push(l),
                        Err(_) => return Ok(false),
                    }
                }
                let first = match leaders.first().and_then(|l| l.as_ref()) {
                    Some(f) => *f,
                    None => return Ok(false),
                };
                if leaders.iter().all(|l| l.as_ref() == Some(&first)) {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        })
        .await?;

        // Re-query to get the actual leader
        match self.check_leader_consensus().await {
            Some(id) => Ok(id.0),
            None => Err(WaitError::ClusterError("leader lost after wait".into())),
        }
    }

    async fn wait_for_replication(&self, timeout: Duration) -> Result<(), WaitError> {
        // Patchbay nodes don't directly expose applied index — use a write-then-read
        // round trip as a proxy for replication convergence.
        let key = format!("__harness_repl_check_{}", rand::random::<u32>());
        PatchbayHarness::write_kv(self, &key, "1")
            .await
            .map_err(|e| WaitError::ClusterError(format!("{e}")))?;

        let handles = self.handles().to_vec();
        let key_clone = key.clone();
        wait_until("patchbay replication converged", timeout, Duration::from_millis(200), move || {
            let hs = handles.clone();
            let k = key_clone.clone();
            async move {
                for (i, h) in hs.iter().enumerate() {
                    match h.read_kv(&k).await {
                        Ok(Some(_)) => {}
                        Ok(None) => return Ok(false),
                        Err(e) => return Err(format!("node {i}: {e}")),
                    }
                }
                Ok(true)
            }
        })
        .await
    }

    async fn write_kv(&self, key: &str, value: &str) -> Result<(), WaitError> {
        PatchbayHarness::write_kv(self, key, value)
            .await
            .map_err(|e| WaitError::ClusterError(format!("{e}")))
    }

    async fn read_kv(&self, key: &str) -> Result<Option<String>, WaitError> {
        // Read from node 0
        PatchbayHarness::read_kv(self, 0, key).await.map_err(|e| WaitError::ClusterError(format!("{e}")))
    }

    fn node_count(&self) -> u32 {
        self.handles.len() as u32
    }
}
