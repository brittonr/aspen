//! File-based peer address lookup for air-gapped and relay-disabled deployments.
//!
//! Implements [`iroh::address_lookup::AddressLookup`] to persist and resolve peer
//! addresses using the local filesystem. Each node publishes its own endpoint
//! address to a JSON file in its data directory. When resolving a peer, sibling
//! data directories are scanned for the peer's address file.
//!
//! Works alongside mDNS, DNS, and Pkarr — iroh queries all address lookup
//! services and uses whichever resolves first.

use std::path::PathBuf;
use std::sync::Arc;

use iroh::EndpointAddr;
use iroh::address_lookup::AddressLookup;
use iroh::address_lookup::EndpointData;
use iroh::address_lookup::EndpointInfo;
use iroh::address_lookup::Error;
use iroh::address_lookup::Item;
use iroh_base::EndpointId;
use n0_future::boxed::BoxStream;
use n0_future::stream::StreamExt;
use n0_future::stream::{self};
use tracing::debug;
use tracing::warn;

/// Callback type for resolving peer addresses from Raft membership.
pub type MembershipResolver = Arc<dyn Fn(EndpointId) -> Option<EndpointAddr> + Send + Sync>;

/// File-based [`AddressLookup`] for cluster peer address recovery.
///
/// - `publish()`: Writes own endpoint address to `<data_dir>/discovery/<key>.json`
/// - `resolve()`: Scans sibling data directories for the peer's file, falls back to Raft membership
///   addresses
#[derive(Clone)]
pub struct ClusterDiscovery {
    /// This node's data directory (e.g., `/tmp/cluster/node1`).
    data_dir: PathBuf,
    /// Parent directory containing all node data dirs (e.g., `/tmp/cluster`).
    cluster_data_dir: Option<PathBuf>,
    /// This node's endpoint ID, set after endpoint creation.
    own_endpoint_id: Arc<std::sync::RwLock<Option<EndpointId>>>,
    /// Fallback: resolve peer addresses from Raft membership.
    membership_resolver: Option<MembershipResolver>,
}

impl std::fmt::Debug for ClusterDiscovery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClusterDiscovery")
            .field("data_dir", &self.data_dir)
            .field("cluster_data_dir", &self.cluster_data_dir)
            .field("has_membership_resolver", &self.membership_resolver.is_some())
            .finish()
    }
}

impl ClusterDiscovery {
    /// Provenance string for log attribution.
    pub const PROVENANCE: &'static str = "cluster_file_discovery";

    /// Maximum number of sibling directories to scan during resolve.
    const MAX_SIBLING_SCAN: u32 = 64;

    /// Create a new `ClusterDiscovery`.
    pub fn new(
        data_dir: PathBuf,
        cluster_data_dir: Option<PathBuf>,
        membership_resolver: Option<MembershipResolver>,
    ) -> Self {
        Self {
            data_dir,
            cluster_data_dir,
            own_endpoint_id: Arc::new(std::sync::RwLock::new(None)),
            membership_resolver,
        }
    }

    /// Set this node's endpoint ID (called after endpoint creation).
    pub fn set_endpoint_id(&self, id: EndpointId) {
        let mut guard = self.own_endpoint_id.write().expect("poisoned");
        *guard = Some(id);
    }

    /// Path to a discovery file for a given endpoint ID.
    fn discovery_file_path(base_dir: &std::path::Path, endpoint_id: &EndpointId) -> PathBuf {
        base_dir.join("discovery").join(format!("{}.json", endpoint_id))
    }

    /// Atomically write an `EndpointAddr` to a discovery file.
    fn write_file(path: &std::path::Path, addr: &EndpointAddr) -> std::io::Result<()> {
        let dir = path.parent().unwrap_or(std::path::Path::new("."));
        std::fs::create_dir_all(dir)?;

        let json =
            serde_json::to_string_pretty(addr).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Atomic write: temp file + rename
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, json)?;
        std::fs::rename(&tmp_path, path)?;

        Ok(())
    }

    /// Read an `EndpointAddr` from a discovery file.
    fn read_file(path: &std::path::Path) -> Option<EndpointAddr> {
        let contents = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    /// Scan sibling data directories for a peer's discovery file.
    fn resolve_from_siblings(&self, endpoint_id: &EndpointId) -> Option<EndpointAddr> {
        let cluster_dir = self.cluster_data_dir.as_ref()?;
        let entries = std::fs::read_dir(cluster_dir).ok()?;

        let target_filename = format!("{}.json", endpoint_id);

        for (scanned, entry) in entries.enumerate() {
            if scanned >= Self::MAX_SIBLING_SCAN as usize {
                break;
            }

            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let discovery_file = path.join("discovery").join(&target_filename);
            if let Some(addr) = Self::read_file(&discovery_file) {
                if addr.id == *endpoint_id {
                    debug!(
                        %endpoint_id,
                        path = %discovery_file.display(),
                        "resolved peer from sibling discovery file"
                    );
                    return Some(addr);
                }
            }
        }

        None
    }

    /// Resolve from Raft membership fallback.
    fn resolve_from_membership(&self, endpoint_id: &EndpointId) -> Option<EndpointAddr> {
        let resolver = self.membership_resolver.as_ref()?;
        let addr = resolver(*endpoint_id)?;
        debug!(
            %endpoint_id,
            addrs = addr.addrs.len(),
            "resolved peer from Raft membership"
        );
        Some(addr)
    }

    /// Publish an endpoint address with an explicit `EndpointAddr`.
    /// Called during bootstrap and shutdown.
    pub fn publish_endpoint_addr(&self, addr: &EndpointAddr) {
        let path = Self::discovery_file_path(&self.data_dir, &addr.id);
        match Self::write_file(&path, addr) {
            Ok(()) => {
                debug!(
                    endpoint_id = %addr.id,
                    path = %path.display(),
                    addrs = addr.addrs.len(),
                    "published discovery file"
                );
            }
            Err(e) => {
                warn!(
                    endpoint_id = %addr.id,
                    path = %path.display(),
                    error = %e,
                    "failed to write discovery file"
                );
            }
        }
    }

    /// Read all peer addresses from sibling discovery directories.
    /// Used during startup to seed iroh's address book.
    pub fn read_all_peers(&self) -> Vec<EndpointAddr> {
        let mut peers = Vec::new();
        let cluster_dir = match &self.cluster_data_dir {
            Some(d) => d,
            None => return peers,
        };

        let entries = match std::fs::read_dir(cluster_dir) {
            Ok(e) => e,
            Err(_) => return peers,
        };

        for (scanned, entry) in entries.enumerate() {
            if scanned >= Self::MAX_SIBLING_SCAN as usize {
                break;
            }

            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let discovery_dir = path.join("discovery");
            if !discovery_dir.is_dir() {
                continue;
            }

            let files = match std::fs::read_dir(&discovery_dir) {
                Ok(f) => f,
                Err(_) => continue,
            };

            for file in files {
                let file = match file {
                    Ok(f) => f,
                    Err(_) => continue,
                };
                let file_path = file.path();
                if file_path.extension().is_some_and(|ext| ext == "json") {
                    if let Some(addr) = Self::read_file(&file_path) {
                        peers.push(addr);
                    }
                }
            }
        }

        peers
    }
}

impl AddressLookup for ClusterDiscovery {
    fn publish(&self, data: &EndpointData) {
        let endpoint_id = {
            let guard = self.own_endpoint_id.read().expect("poisoned");
            match *guard {
                Some(id) => id,
                None => {
                    debug!("publish called before endpoint_id set, skipping");
                    return;
                }
            }
        };

        // Reconstruct EndpointAddr from EndpointData
        let addr = EndpointAddr::from_parts(endpoint_id, data.addrs().cloned());
        let path = Self::discovery_file_path(&self.data_dir, &endpoint_id);
        match Self::write_file(&path, &addr) {
            Ok(()) => {
                debug!(
                    %endpoint_id,
                    path = %path.display(),
                    "published discovery file via AddressLookup"
                );
            }
            Err(e) => {
                warn!(
                    %endpoint_id,
                    path = %path.display(),
                    error = %e,
                    "failed to write discovery file"
                );
            }
        }
    }

    fn resolve(&self, endpoint_id: EndpointId) -> Option<BoxStream<Result<Item, Error>>> {
        let addr = self.resolve_from_siblings(&endpoint_id).or_else(|| self.resolve_from_membership(&endpoint_id))?;

        let info = EndpointInfo::from(addr);
        let item = Item::new(info, Self::PROVENANCE, None);

        Some(stream::iter(Some(Ok(item))).boxed())
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use iroh::TransportAddr;

    use super::*;

    fn make_endpoint_id(seed: u8) -> EndpointId {
        let key = iroh_base::SecretKey::from_bytes(&[seed; 32]);
        key.public()
    }

    fn make_addr(seed: u8, port: u16) -> EndpointAddr {
        let id = make_endpoint_id(seed);
        let socket: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
        EndpointAddr::from_parts(id, [TransportAddr::Ip(socket)])
    }

    #[test]
    fn test_publish_writes_file_and_read_back() {
        let tmp = tempfile::tempdir().unwrap();
        let node1_dir = tmp.path().join("node1");
        std::fs::create_dir_all(&node1_dir).unwrap();

        let discovery = ClusterDiscovery::new(node1_dir.clone(), None, None);
        let addr = make_addr(1, 5000);
        discovery.publish_endpoint_addr(&addr);

        let file = ClusterDiscovery::discovery_file_path(&node1_dir, &addr.id);
        assert!(file.exists(), "discovery file should exist after publish");

        let read_back = ClusterDiscovery::read_file(&file).unwrap();
        assert_eq!(read_back.id, addr.id);
        assert_eq!(read_back.addrs, addr.addrs);
    }

    #[test]
    fn test_resolve_from_siblings() {
        let tmp = tempfile::tempdir().unwrap();
        let node1_dir = tmp.path().join("node1");
        let node2_dir = tmp.path().join("node2");
        std::fs::create_dir_all(&node1_dir).unwrap();
        std::fs::create_dir_all(&node2_dir).unwrap();

        let node2_disc = ClusterDiscovery::new(node2_dir, None, None);
        let node2_addr = make_addr(2, 6000);
        node2_disc.publish_endpoint_addr(&node2_addr);

        let node1_disc = ClusterDiscovery::new(node1_dir, Some(tmp.path().to_path_buf()), None);
        let resolved = node1_disc.resolve_from_siblings(&node2_addr.id);
        assert!(resolved.is_some(), "should resolve node2 from sibling dir");
        let resolved = resolved.unwrap();
        assert_eq!(resolved.id, node2_addr.id);
        assert_eq!(resolved.addrs, node2_addr.addrs);
    }

    #[test]
    fn test_resolve_falls_back_to_membership() {
        let tmp = tempfile::tempdir().unwrap();
        let node1_dir = tmp.path().join("node1");
        std::fs::create_dir_all(&node1_dir).unwrap();

        let node2_id = make_endpoint_id(2);
        let node2_addr = make_addr(2, 7000);
        let membership_addr = node2_addr.clone();

        let resolver: MembershipResolver = Arc::new(move |id| {
            if id == node2_id {
                Some(membership_addr.clone())
            } else {
                None
            }
        });

        let discovery = ClusterDiscovery::new(node1_dir, Some(tmp.path().to_path_buf()), Some(resolver));

        assert!(discovery.resolve_from_siblings(&node2_id).is_none());
        let from_membership = discovery.resolve_from_membership(&node2_id);
        assert!(from_membership.is_some());
        assert_eq!(from_membership.unwrap().id, node2_id);
    }

    #[test]
    fn test_read_all_peers() {
        let tmp = tempfile::tempdir().unwrap();

        for seed in 1u8..=3 {
            let node_dir = tmp.path().join(format!("node{}", seed));
            let disc = ClusterDiscovery::new(node_dir, None, None);
            let addr = make_addr(seed, 5000 + seed as u16);
            disc.publish_endpoint_addr(&addr);
        }

        let node1_disc = ClusterDiscovery::new(tmp.path().join("node1"), Some(tmp.path().to_path_buf()), None);
        let peers = node1_disc.read_all_peers();
        assert_eq!(peers.len(), 3, "should find all 3 discovery files");
    }

    #[test]
    fn test_publish_via_address_lookup_trait() {
        let tmp = tempfile::tempdir().unwrap();
        let node_dir = tmp.path().join("node1");
        std::fs::create_dir_all(&node_dir).unwrap();

        let discovery = ClusterDiscovery::new(node_dir.clone(), None, None);
        let addr = make_addr(1, 5000);
        let data = EndpointData::new(addr.addrs.iter().cloned());

        // Before setting endpoint_id, publish is a no-op
        discovery.publish(&data);
        let file = ClusterDiscovery::discovery_file_path(&node_dir, &addr.id);
        assert!(!file.exists());

        // After setting endpoint_id, publish writes the file
        discovery.set_endpoint_id(addr.id);
        discovery.publish(&data);
        assert!(file.exists());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let addr = make_addr(42, 9999);
        let json = serde_json::to_string_pretty(&addr).unwrap();
        let recovered: EndpointAddr = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.id, addr.id);
        assert_eq!(recovered.addrs, addr.addrs);
    }
}
