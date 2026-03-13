//! Patchbay-based network simulation testing for Aspen.
//!
//! This crate provides a test harness that spawns real Aspen nodes inside
//! patchbay network namespaces with configurable topologies (NAT types,
//! link conditions, regions). Tests exercise real iroh QUIC connections
//! through simulated network infrastructure.
//!
//! # Testing Layers
//!
//! This fills the gap between two existing testing approaches:
//!
//! - **madsim**: Deterministic simulation with fake networking
//! - **NixOS VMs**: Full-system tests with real but coarse network control
//! - **patchbay** (this crate): Real iroh connections through simulated topologies with
//!   fine-grained fault injection
//!
//! # Requirements
//!
//! - Linux with unprivileged user namespaces (`kernel.unprivileged_userns_clone=1`)
//! - `nft` and `tc` in PATH (provided by nftables and iproute2)
//! - No root privileges required
//!
//! # Example
//!
//! ```ignore
//! use aspen_testing_patchbay::prelude::*;
//!
//! #[tokio::test]
//! async fn test_cluster_through_nat() {
//!     skip_unless_patchbay!();
//!     let harness = PatchbayHarness::three_node_home_nat().await.unwrap();
//!     harness.init_cluster().await.unwrap();
//!     harness.write_kv("key1", "value1").await.unwrap();
//!     let val = harness.read_kv(2, "key1").await.unwrap();
//!     assert_eq!(val, Some("value1".to_string()));
//! }
//! ```

pub mod harness;
pub mod node;
pub mod skip;
pub mod topologies;
pub mod transport;

/// Skip the current test if patchbay prerequisites are not met.
///
/// Prints a diagnostic message and returns early (success) rather than failing.
/// Use at the top of any patchbay test function.
///
/// ```ignore
/// #[tokio::test]
/// async fn test_something() {
///     skip_unless_patchbay!();
///     // ... rest of test
/// }
/// ```
#[macro_export]
macro_rules! skip_unless_patchbay {
    () => {
        if !$crate::skip::patchbay_available() {
            let reason = $crate::skip::patchbay_unavailable_reason().unwrap_or_else(|| "unknown reason".to_string());
            eprintln!("skipping patchbay test: {} (this is not a failure)", reason);
            return;
        }
    };
}

/// Convenience prelude for tests.
pub mod prelude {
    pub use patchbay::Device;
    pub use patchbay::Lab;
    pub use patchbay::LinkCondition;
    pub use patchbay::LinkLimits;
    pub use patchbay::Nat;
    pub use patchbay::NatV6Mode;
    pub use patchbay::Region;
    pub use patchbay::RegionLink;
    pub use patchbay::Router;
    pub use patchbay::RouterPreset;

    pub use crate::harness::PatchbayHarness;
    pub use crate::node::NodeHandle;
    pub use crate::skip::patchbay_available;
    pub use crate::skip_unless_patchbay;
}
