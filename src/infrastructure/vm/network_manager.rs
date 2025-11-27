//! VM Network Management
//!
//! Handles IP address allocation for VMs

use uuid::Uuid;

/// Manages network resources for VMs
pub struct VmNetworkManager;

impl VmNetworkManager {
    /// Create a new network manager
    pub fn new() -> Self {
        Self
    }

    /// Allocate an IP address for the VM
    ///
    /// Simple static allocation: 192.168.100.X where X is based on a hash of the VM ID
    /// Uses hash to generate last octet (2-254 to avoid network/broadcast addresses)
    pub fn allocate_ip_address(&self, vm_id: Uuid) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        vm_id.hash(&mut hasher);
        let hash = hasher.finish();

        // Use hash to generate last octet (2-254 to avoid network/broadcast addresses)
        let last_octet = 2 + (hash % 253) as u8;
        format!("192.168.100.{}", last_octet)
    }
}

impl Default for VmNetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_allocation_deterministic() {
        let network_mgr = VmNetworkManager::new();
        let vm_id = Uuid::new_v4();

        let ip1 = network_mgr.allocate_ip_address(vm_id);
        let ip2 = network_mgr.allocate_ip_address(vm_id);

        // Same VM ID should always get same IP
        assert_eq!(ip1, ip2);
    }

    #[test]
    fn test_ip_allocation_range() {
        let network_mgr = VmNetworkManager::new();

        for _ in 0..100 {
            let vm_id = Uuid::new_v4();
            let ip = network_mgr.allocate_ip_address(vm_id);

            // Should always be in 192.168.100.x range
            assert!(ip.starts_with("192.168.100."));

            // Extract last octet
            let last_octet: u8 = ip.split('.').last().unwrap().parse().unwrap();

            // Should be in valid range (2-254)
            assert!(last_octet >= 2 && last_octet <= 254);
        }
    }
}
