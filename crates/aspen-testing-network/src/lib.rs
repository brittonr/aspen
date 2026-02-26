//! Network testing utilities for Aspen.
//!
//! This crate provides utilities for network-based integration testing,
//! including fault injection, TAP/bridge management, and VM-based testing
//! infrastructure.
//!
//! # Features
//!
//! - `vm`: Enable VM management utilities (requires reqwest for health checks)
//! - `tap`: Enable TAP device and bridge utilities (requires root privileges)
//!
//! # Modules
//!
//! - [`fault_injection`]: Network partition, latency, and packet loss simulation
//! - [`network_utils`]: TAP device and bridge management
//! - [`vm_manager`]: Cloud Hypervisor VM management (requires `vm` feature)
//!
//! # Example
//!
//! ```ignore
//! use aspen_testing_network::fault_injection::{NetworkPartition, LatencyInjection};
//!
//! // Create a network partition isolating node 1 from nodes 2 and 3
//! let partition = NetworkPartition::create("10.100.0.11", &["10.100.0.12", "10.100.0.13"])?;
//!
//! // Run tests while partition is active...
//!
//! // Heal the partition (or let it auto-heal on drop)
//! partition.heal()?;
//! ```

pub mod fault_injection;
pub mod network_utils;

#[cfg(feature = "vm")]
pub mod vm_manager;

// Re-export key types
pub use fault_injection::AsymmetricPartition;
pub use fault_injection::FaultError;
pub use fault_injection::FaultScenario;
pub use fault_injection::LatencyInjection;
pub use fault_injection::NetworkPartition;
pub use fault_injection::PacketLossInjection;
pub use fault_injection::PartitionDirection;
pub use network_utils::NetworkBridge;
pub use network_utils::NetworkError;
pub use network_utils::TapDevice;
#[cfg(feature = "vm")]
pub use vm_manager::ManagedVm;
#[cfg(feature = "vm")]
pub use vm_manager::NetworkConfig;
#[cfg(feature = "vm")]
pub use vm_manager::VmConfig;
#[cfg(feature = "vm")]
pub use vm_manager::VmManager;
#[cfg(feature = "vm")]
pub use vm_manager::VmManagerError;
#[cfg(feature = "vm")]
pub use vm_manager::VmState;
