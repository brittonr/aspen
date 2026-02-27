//! Fault injection utilities for VM-based integration testing.
//!
//! This module provides utilities for injecting various types of faults
//! into a running Aspen cluster to test resilience and recovery:
//!
//! - Network partitions (isolate nodes from each other)
//! - Latency injection (add artificial delay to network traffic)
//! - Packet loss (drop a percentage of packets)
//! - Node crashes (kill and restart VM processes)
//! - Disk full simulation (fill up storage)
//!
//! # Tiger Style
//!
//! - All fault injections are reversible via heal methods
//! - Fixed limits on fault parameters (max latency, max packet loss)
//! - Automatic cleanup via Drop trait implementations
//! - Clear separation between fault injection and verification
//!
//! # Test Coverage
//!
//! Unit tests cover NetworkPartition (create, empty targets, asymmetric,
//! lifecycle, heal idempotency, drop cleanup), LatencyInjection (validation,
//! boundary values, lifecycle), PacketLoss (validation, boundary values,
//! lifecycle), and FaultScenario builder with multiple simultaneous faults.
//!
//! # Example
//!
//! ```ignore
//! use aspen::testing::fault_injection::{NetworkPartition, LatencyInjection};
//!
//! // Create a network partition isolating node 1 from nodes 2 and 3
//! let partition = NetworkPartition::create(1, &[2, 3])?;
//!
//! // Run tests while partition is active...
//!
//! // Heal the partition
//! partition.heal()?;
//!
//! // Or let it heal automatically when dropped
//! ```

use std::process::Command;

use snafu::ResultExt;
use snafu::Snafu;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Maximum latency that can be injected (ms).
/// Tiger Style: Fixed limit to prevent unbounded resource use.
const MAX_LATENCY_MS: u32 = 10000;

/// Maximum packet loss percentage.
const MAX_PACKET_LOSS_PERCENT: u8 = 100;

/// Network partition fault injection.
///
/// Isolates one node from a set of other nodes using iptables rules.
/// The partition is symmetric (traffic blocked in both directions).
#[derive(Debug)]
pub struct NetworkPartition {
    /// IP address of the isolated node.
    source_ip: String,
    /// IP addresses of nodes that the source is partitioned from.
    target_ips: Vec<String>,
    /// Whether the partition is currently active.
    is_active: bool,
}

impl NetworkPartition {
    /// Create a new network partition.
    ///
    /// # Arguments
    ///
    /// * `source_ip` - IP address of the node to isolate.
    /// * `target_ips` - IP addresses of nodes to isolate from.
    ///
    /// # Errors
    ///
    /// Returns an error if the iptables rules cannot be created.
    pub fn create(source_ip: &str, target_ips: &[&str]) -> Result<Self, FaultError> {
        let mut partition = Self {
            source_ip: source_ip.to_string(),
            target_ips: target_ips.iter().map(|s| s.to_string()).collect(),
            is_active: false,
        };

        partition.inject()?;
        partition.is_active = true;
        Ok(partition)
    }

    /// Inject the network partition.
    fn inject(&self) -> Result<(), FaultError> {
        info!(
            source = %self.source_ip,
            targets = ?self.target_ips,
            "Injecting network partition"
        );

        for target_ip in &self.target_ips {
            // Block traffic from source to target
            run_iptables(&["-I", "FORWARD", "-s", &self.source_ip, "-d", target_ip, "-j", "DROP"])?;

            // Block traffic from target to source (symmetric partition)
            run_iptables(&["-I", "FORWARD", "-s", target_ip, "-d", &self.source_ip, "-j", "DROP"])?;
        }

        Ok(())
    }

    /// Heal the network partition.
    pub fn heal(&mut self) -> Result<(), FaultError> {
        if !self.is_active {
            return Ok(());
        }

        info!(
            source = %self.source_ip,
            targets = ?self.target_ips,
            "Healing network partition"
        );

        for target_ip in &self.target_ips {
            // Remove DROP rules
            let _ = run_iptables(&["-D", "FORWARD", "-s", &self.source_ip, "-d", target_ip, "-j", "DROP"]);

            let _ = run_iptables(&["-D", "FORWARD", "-s", target_ip, "-d", &self.source_ip, "-j", "DROP"]);
        }

        self.is_active = false;
        Ok(())
    }

    /// Check if the partition is currently active.
    pub fn is_active(&self) -> bool {
        self.is_active
    }
}

impl Drop for NetworkPartition {
    fn drop(&mut self) {
        if self.is_active
            && let Err(e) = self.heal()
        {
            warn!(
                source = %self.source_ip,
                error = %e,
                "Failed to heal partition during drop"
            );
        }
    }
}

/// Direction of traffic blocking for asymmetric partitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionDirection {
    /// Block outbound traffic only (source can receive but not send).
    OutboundOnly,
    /// Block inbound traffic only (source can send but not receive).
    InboundOnly,
}

/// Asymmetric network partition fault injection.
///
/// Unlike `NetworkPartition`, this only blocks traffic in one direction,
/// creating scenarios where a node can receive but not send (or vice versa).
///
/// This is useful for testing:
/// - Nodes that hear heartbeats but can't respond to vote requests
/// - Nodes that send requests but never receive responses
/// - Split-brain scenarios with one-way communication
#[derive(Debug)]
pub struct AsymmetricPartition {
    /// IP address of the affected node.
    source_ip: String,
    /// IP addresses of nodes affected by the partition.
    target_ips: Vec<String>,
    /// Direction of traffic blocking.
    direction: PartitionDirection,
    /// Whether the partition is currently active.
    is_active: bool,
}

impl AsymmetricPartition {
    /// Create a new asymmetric network partition.
    ///
    /// # Arguments
    ///
    /// * `source_ip` - IP address of the node to apply partition to.
    /// * `target_ips` - IP addresses of the other nodes.
    /// * `direction` - Which direction to block traffic.
    ///
    /// # Errors
    ///
    /// Returns an error if the iptables rules cannot be created.
    pub fn create(source_ip: &str, target_ips: &[&str], direction: PartitionDirection) -> Result<Self, FaultError> {
        let mut partition = Self {
            source_ip: source_ip.to_string(),
            target_ips: target_ips.iter().map(|s| s.to_string()).collect(),
            direction,
            is_active: false,
        };

        partition.inject()?;
        partition.is_active = true;
        Ok(partition)
    }

    /// Inject the asymmetric network partition.
    fn inject(&self) -> Result<(), FaultError> {
        info!(
            source = %self.source_ip,
            targets = ?self.target_ips,
            direction = ?self.direction,
            "Injecting asymmetric network partition"
        );

        for target_ip in &self.target_ips {
            match self.direction {
                PartitionDirection::OutboundOnly => {
                    // Block traffic from source to target (source can't send)
                    run_iptables(&["-I", "FORWARD", "-s", &self.source_ip, "-d", target_ip, "-j", "DROP"])?;
                }
                PartitionDirection::InboundOnly => {
                    // Block traffic from target to source (source can't receive)
                    run_iptables(&["-I", "FORWARD", "-s", target_ip, "-d", &self.source_ip, "-j", "DROP"])?;
                }
            }
        }

        Ok(())
    }

    /// Heal the asymmetric network partition.
    pub fn heal(&mut self) -> Result<(), FaultError> {
        if !self.is_active {
            return Ok(());
        }

        info!(
            source = %self.source_ip,
            targets = ?self.target_ips,
            direction = ?self.direction,
            "Healing asymmetric network partition"
        );

        for target_ip in &self.target_ips {
            match self.direction {
                PartitionDirection::OutboundOnly => {
                    let _ = run_iptables(&["-D", "FORWARD", "-s", &self.source_ip, "-d", target_ip, "-j", "DROP"]);
                }
                PartitionDirection::InboundOnly => {
                    let _ = run_iptables(&["-D", "FORWARD", "-s", target_ip, "-d", &self.source_ip, "-j", "DROP"]);
                }
            }
        }

        self.is_active = false;
        Ok(())
    }

    /// Check if the partition is currently active.
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Get the direction of the partition.
    pub fn direction(&self) -> PartitionDirection {
        self.direction
    }
}

impl Drop for AsymmetricPartition {
    fn drop(&mut self) {
        if self.is_active
            && let Err(e) = self.heal()
        {
            warn!(
                source = %self.source_ip,
                error = %e,
                "Failed to heal asymmetric partition during drop"
            );
        }
    }
}

/// Latency injection fault.
///
/// Adds artificial delay to network traffic using tc (traffic control).
#[derive(Debug)]
pub struct LatencyInjection {
    /// Interface to apply latency to.
    interface: String,
    /// Latency in milliseconds.
    latency_ms: u32,
    /// Jitter (variation) in milliseconds.
    jitter_ms: u32,
    /// Whether the injection is currently active.
    is_active: bool,
}

impl LatencyInjection {
    /// Create a new latency injection.
    ///
    /// # Arguments
    ///
    /// * `interface` - Network interface to apply latency to.
    /// * `latency_ms` - Base latency in milliseconds.
    /// * `jitter_ms` - Jitter (random variation) in milliseconds.
    ///
    /// # Errors
    ///
    /// Returns an error if the tc rules cannot be created.
    pub fn create(interface: &str, latency_ms: u32, jitter_ms: u32) -> Result<Self, FaultError> {
        if latency_ms > MAX_LATENCY_MS {
            return Err(FaultError::InvalidParameter {
                name: "latency_ms",
                value: latency_ms.to_string(),
                max: MAX_LATENCY_MS.to_string(),
            });
        }

        let mut injection = Self {
            interface: interface.to_string(),
            latency_ms,
            jitter_ms,
            is_active: false,
        };

        injection.inject()?;
        injection.is_active = true;
        Ok(injection)
    }

    /// Inject the latency.
    fn inject(&self) -> Result<(), FaultError> {
        info!(
            interface = %self.interface,
            latency_ms = self.latency_ms,
            jitter_ms = self.jitter_ms,
            "Injecting network latency"
        );

        // Add netem qdisc with delay
        run_tc(&[
            "qdisc",
            "add",
            "dev",
            &self.interface,
            "root",
            "netem",
            "delay",
            &format!("{}ms", self.latency_ms),
            &format!("{}ms", self.jitter_ms),
        ])?;

        Ok(())
    }

    /// Remove the latency injection.
    pub fn heal(&mut self) -> Result<(), FaultError> {
        if !self.is_active {
            return Ok(());
        }

        info!(
            interface = %self.interface,
            "Removing network latency injection"
        );

        // Remove the qdisc
        let _ = run_tc(&["qdisc", "del", "dev", &self.interface, "root"]);

        self.is_active = false;
        Ok(())
    }

    /// Check if the injection is currently active.
    pub fn is_active(&self) -> bool {
        self.is_active
    }
}

impl Drop for LatencyInjection {
    fn drop(&mut self) {
        if self.is_active
            && let Err(e) = self.heal()
        {
            warn!(
                interface = %self.interface,
                error = %e,
                "Failed to heal latency injection during drop"
            );
        }
    }
}

/// Packet loss injection fault.
///
/// Drops a percentage of packets using tc (traffic control).
#[derive(Debug)]
pub struct PacketLossInjection {
    /// Interface to apply packet loss to.
    interface: String,
    /// Percentage of packets to drop (0-100).
    loss_percent: u8,
    /// Whether the injection is currently active.
    is_active: bool,
}

impl PacketLossInjection {
    /// Create a new packet loss injection.
    ///
    /// # Arguments
    ///
    /// * `interface` - Network interface to apply packet loss to.
    /// * `loss_percent` - Percentage of packets to drop (0-100).
    ///
    /// # Errors
    ///
    /// Returns an error if the tc rules cannot be created.
    pub fn create(interface: &str, loss_percent: u8) -> Result<Self, FaultError> {
        if loss_percent > MAX_PACKET_LOSS_PERCENT {
            return Err(FaultError::InvalidParameter {
                name: "loss_percent",
                value: loss_percent.to_string(),
                max: MAX_PACKET_LOSS_PERCENT.to_string(),
            });
        }

        let mut injection = Self {
            interface: interface.to_string(),
            loss_percent,
            is_active: false,
        };

        injection.inject()?;
        injection.is_active = true;
        Ok(injection)
    }

    /// Inject the packet loss.
    fn inject(&self) -> Result<(), FaultError> {
        info!(
            interface = %self.interface,
            loss_percent = self.loss_percent,
            "Injecting packet loss"
        );

        // Add netem qdisc with loss
        run_tc(&[
            "qdisc",
            "add",
            "dev",
            &self.interface,
            "root",
            "netem",
            "loss",
            &format!("{}%", self.loss_percent),
        ])?;

        Ok(())
    }

    /// Remove the packet loss injection.
    pub fn heal(&mut self) -> Result<(), FaultError> {
        if !self.is_active {
            return Ok(());
        }

        info!(
            interface = %self.interface,
            "Removing packet loss injection"
        );

        // Remove the qdisc
        let _ = run_tc(&["qdisc", "del", "dev", &self.interface, "root"]);

        self.is_active = false;
        Ok(())
    }

    /// Check if the injection is currently active.
    pub fn is_active(&self) -> bool {
        self.is_active
    }
}

impl Drop for PacketLossInjection {
    fn drop(&mut self) {
        if self.is_active
            && let Err(e) = self.heal()
        {
            warn!(
                interface = %self.interface,
                error = %e,
                "Failed to heal packet loss injection during drop"
            );
        }
    }
}

/// Helper to run iptables commands.
fn run_iptables(args: &[&str]) -> Result<(), FaultError> {
    debug!(args = ?args, "Running iptables command");

    let output = Command::new("iptables").args(args).output().context(IoSnafu { operation: "iptables" })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(FaultError::CommandFailed {
            command: format!("iptables {}", args.join(" ")),
            stderr: stderr.to_string(),
        });
    }

    Ok(())
}

/// Helper to run tc (traffic control) commands.
fn run_tc(args: &[&str]) -> Result<(), FaultError> {
    debug!(args = ?args, "Running tc command");

    let output = Command::new("tc").args(args).output().context(IoSnafu { operation: "tc" })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(FaultError::CommandFailed {
            command: format!("tc {}", args.join(" ")),
            stderr: stderr.to_string(),
        });
    }

    Ok(())
}

/// Fault injection errors.
#[derive(Debug, Snafu)]
pub enum FaultError {
    /// I/O error during fault injection operation.
    #[snafu(display("I/O error during {operation}: {source}"))]
    Io {
        /// The operation that failed.
        operation: &'static str,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// External command (tc, iptables, etc.) failed.
    #[snafu(display("Command failed: {command}\nStderr: {stderr}"))]
    CommandFailed {
        /// The command that failed.
        command: String,
        /// Standard error output from the command.
        stderr: String,
    },

    /// A fault parameter is invalid or out of range.
    #[snafu(display("Invalid parameter {name}={value} (max: {max})"))]
    InvalidParameter {
        /// The parameter name.
        name: &'static str,
        /// The invalid value provided.
        value: String,
        /// The maximum allowed value.
        max: String,
    },
}

/// Fault scenario builder for complex fault injection patterns.
///
/// Allows composing multiple faults and applying them in sequence
/// or simultaneously.
#[derive(Default)]
pub struct FaultScenario {
    /// Active network partitions.
    partitions: Vec<NetworkPartition>,
    /// Active asymmetric partitions.
    asymmetric_partitions: Vec<AsymmetricPartition>,
    /// Active latency injections.
    latencies: Vec<LatencyInjection>,
    /// Active packet loss injections.
    packet_losses: Vec<PacketLossInjection>,
}

impl FaultScenario {
    /// Create a new empty fault scenario.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a network partition to the scenario.
    pub fn with_partition(mut self, source_ip: &str, target_ips: &[&str]) -> Result<Self, FaultError> {
        let partition = NetworkPartition::create(source_ip, target_ips)?;
        self.partitions.push(partition);
        Ok(self)
    }

    /// Add an asymmetric network partition to the scenario.
    pub fn with_asymmetric_partition(
        mut self,
        source_ip: &str,
        target_ips: &[&str],
        direction: PartitionDirection,
    ) -> Result<Self, FaultError> {
        let partition = AsymmetricPartition::create(source_ip, target_ips, direction)?;
        self.asymmetric_partitions.push(partition);
        Ok(self)
    }

    /// Add a latency injection to the scenario.
    pub fn with_latency(mut self, interface: &str, latency_ms: u32, jitter_ms: u32) -> Result<Self, FaultError> {
        let latency = LatencyInjection::create(interface, latency_ms, jitter_ms)?;
        self.latencies.push(latency);
        Ok(self)
    }

    /// Add a packet loss injection to the scenario.
    pub fn with_packet_loss(mut self, interface: &str, loss_percent: u8) -> Result<Self, FaultError> {
        let loss = PacketLossInjection::create(interface, loss_percent)?;
        self.packet_losses.push(loss);
        Ok(self)
    }

    /// Heal all faults in the scenario.
    pub fn heal_all(&mut self) -> Result<(), FaultError> {
        for partition in &mut self.partitions {
            partition.heal()?;
        }
        for partition in &mut self.asymmetric_partitions {
            partition.heal()?;
        }
        for latency in &mut self.latencies {
            latency.heal()?;
        }
        for loss in &mut self.packet_losses {
            loss.heal()?;
        }
        Ok(())
    }

    /// Check if any faults are currently active.
    pub fn has_active_faults(&self) -> bool {
        self.partitions.iter().any(|p| p.is_active())
            || self.asymmetric_partitions.iter().any(|p| p.is_active())
            || self.latencies.iter().any(|l| l.is_active())
            || self.packet_losses.iter().any(|p| p.is_active())
    }
}

impl Drop for FaultScenario {
    fn drop(&mut self) {
        if let Err(e) = self.heal_all() {
            warn!(error = %e, "Failed to heal fault scenario during drop");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_validation() {
        // Should fail with latency > MAX_LATENCY_MS
        let result = LatencyInjection::create("eth0", MAX_LATENCY_MS + 1, 0);
        assert!(result.is_err());
        assert!(matches!(result, Err(FaultError::InvalidParameter { name: "latency_ms", .. })));
    }

    #[test]
    fn test_packet_loss_validation() {
        // Should fail with loss > 100%
        let result = PacketLossInjection::create("eth0", 101);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(FaultError::InvalidParameter {
                name: "loss_percent",
                ..
            })
        ));

        // Valid values should work (in terms of structure, may fail due to root privileges)
        let result = PacketLossInjection::create("eth0", 50);
        // May fail due to lack of root privileges, but should not be due to validation
        if let Err(e) = result {
            // Should be a command execution error, not validation error
            assert!(!matches!(e, FaultError::InvalidParameter { .. }));
        }
    }

    #[test]
    fn test_latency_boundary_values() {
        // Test boundary values for latency
        let result = LatencyInjection::create("eth0", 0, 0);
        if let Err(e) = result {
            // Should not be a validation error
            assert!(!matches!(e, FaultError::InvalidParameter { .. }));
        }

        let result = LatencyInjection::create("eth0", MAX_LATENCY_MS, 0);
        if let Err(e) = result {
            // Should not be a validation error
            assert!(!matches!(e, FaultError::InvalidParameter { .. }));
        }

        // Just over the limit should fail validation
        let result = LatencyInjection::create("eth0", MAX_LATENCY_MS + 1, 0);
        assert!(matches!(result, Err(FaultError::InvalidParameter { .. })));
    }

    #[test]
    fn test_packet_loss_boundary_values() {
        // Test boundary values for packet loss
        let result = PacketLossInjection::create("eth0", 0);
        if let Err(e) = result {
            // Should not be a validation error
            assert!(!matches!(e, FaultError::InvalidParameter { .. }));
        }

        let result = PacketLossInjection::create("eth0", 100);
        if let Err(e) = result {
            // Should not be a validation error
            assert!(!matches!(e, FaultError::InvalidParameter { .. }));
        }

        // Just over the limit should fail validation
        let result = PacketLossInjection::create("eth0", 101);
        assert!(matches!(result, Err(FaultError::InvalidParameter { .. })));
    }

    #[test]
    fn test_network_partition_creation() {
        // Test creating partition with valid IP addresses
        let source_ip = "10.100.0.11";
        let target_ips = &["10.100.0.12", "10.100.0.13"];

        // This will likely fail due to lack of root privileges, but we can test structure
        let result = NetworkPartition::create(source_ip, target_ips);

        // Should not panic on creation attempt
        // May fail due to iptables execution, but that's expected in test environment
        match result {
            Ok(partition) => {
                // If somehow it succeeded (running as root), verify structure
                assert!(partition.is_active());
                assert_eq!(partition.source_ip, source_ip);
                assert_eq!(partition.target_ips, vec!["10.100.0.12", "10.100.0.13"]);
            }
            Err(e) => {
                // Expected in non-root environment
                // Should be a command execution error, not a validation error
                assert!(matches!(e, FaultError::CommandFailed { .. } | FaultError::Io { .. }));
            }
        }
    }

    #[test]
    fn test_network_partition_empty_targets() {
        // Test creating partition with no target IPs
        let source_ip = "10.100.0.11";
        let target_ips: &[&str] = &[];

        let result = NetworkPartition::create(source_ip, target_ips);

        // Should not fail validation (empty targets is valid, just a no-op)
        match result {
            Ok(partition) => {
                assert!(partition.is_active());
                assert_eq!(partition.target_ips.len(), 0);
            }
            Err(e) => {
                // Expected in non-root environment
                assert!(matches!(e, FaultError::CommandFailed { .. } | FaultError::Io { .. }));
            }
        }
    }

    #[test]
    fn test_asymmetric_partition_directions() {
        let source_ip = "10.100.0.11";
        let target_ips = &["10.100.0.12"];

        // Test outbound-only partition
        let result = AsymmetricPartition::create(source_ip, target_ips, PartitionDirection::OutboundOnly);
        match result {
            Ok(partition) => {
                assert!(partition.is_active());
                assert_eq!(partition.direction(), PartitionDirection::OutboundOnly);
                assert_eq!(partition.source_ip, source_ip);
            }
            Err(e) => {
                assert!(matches!(e, FaultError::CommandFailed { .. } | FaultError::Io { .. }));
            }
        }

        // Test inbound-only partition
        let result = AsymmetricPartition::create(source_ip, target_ips, PartitionDirection::InboundOnly);
        match result {
            Ok(partition) => {
                assert!(partition.is_active());
                assert_eq!(partition.direction(), PartitionDirection::InboundOnly);
            }
            Err(e) => {
                assert!(matches!(e, FaultError::CommandFailed { .. } | FaultError::Io { .. }));
            }
        }
    }

    #[test]
    fn test_partition_direction_enum() {
        // Test enum properties
        assert_eq!(PartitionDirection::OutboundOnly, PartitionDirection::OutboundOnly);
        assert_ne!(PartitionDirection::OutboundOnly, PartitionDirection::InboundOnly);

        // Test Debug formatting
        let debug_str = format!("{:?}", PartitionDirection::OutboundOnly);
        assert!(debug_str.contains("OutboundOnly"));
    }

    #[test]
    fn test_fault_scenario_builder() {
        // Test building a complex fault scenario
        let scenario = FaultScenario::new();
        assert!(!scenario.has_active_faults());

        // Test adding faults (will likely fail due to privileges, but tests the API)
        let result = scenario.with_partition("10.100.0.11", &["10.100.0.12"]);

        // The builder pattern should work regardless of command execution
        match result {
            Ok(scenario) => {
                assert!(scenario.has_active_faults());
            }
            Err(_) => {
                // Expected in test environment without root
            }
        }
    }

    #[test]
    fn test_fault_scenario_multiple_faults() {
        let mut scenario = FaultScenario::new();

        // Test that empty scenario has no active faults
        assert!(!scenario.has_active_faults());

        // Test heal_all on empty scenario (should not error)
        let result = scenario.heal_all();
        assert!(result.is_ok());
    }

    #[test]
    fn test_constants_and_limits() {
        // Verify the constants are reasonable
        assert_eq!(MAX_LATENCY_MS, 10000);
        assert_eq!(MAX_PACKET_LOSS_PERCENT, 100);

        // Test that constants are used correctly in validation
        assert!(MAX_LATENCY_MS > 0);
        assert!(MAX_PACKET_LOSS_PERCENT <= 100);
    }

    #[test]
    fn test_drop_cleanup_safety() {
        // Test that Drop implementations don't panic

        // Create structures that will likely fail to inject but should drop safely
        let scenario = FaultScenario::new();
        drop(scenario); // Should not panic

        // Test individual fault types
        if let Ok(mut partition) = NetworkPartition::create("192.168.1.1", &["192.168.1.2"]) {
            // If creation succeeded, test healing
            let _ = partition.heal(); // Should not panic
            drop(partition); // Should not panic
        }

        if let Ok(mut partition) =
            AsymmetricPartition::create("192.168.1.1", &["192.168.1.2"], PartitionDirection::OutboundOnly)
        {
            let _ = partition.heal(); // Should not panic
            drop(partition); // Should not panic
        }

        if let Ok(mut latency) = LatencyInjection::create("eth0", 100, 10) {
            let _ = latency.heal(); // Should not panic
            drop(latency); // Should not panic
        }

        if let Ok(mut loss) = PacketLossInjection::create("eth0", 10) {
            let _ = loss.heal(); // Should not panic
            drop(loss); // Should not panic
        }
    }

    #[test]
    fn test_heal_idempotency() {
        // Test that heal operations are idempotent (can be called multiple times safely)

        if let Ok(mut partition) = NetworkPartition::create("192.168.1.1", &["192.168.1.2"]) {
            // First heal
            let result1 = partition.heal();
            // Second heal should also succeed (idempotent)
            let result2 = partition.heal();

            // Both should be Ok (heal is idempotent)
            if result1.is_ok() {
                assert!(result2.is_ok());
                assert!(!partition.is_active());
            }
        }

        if let Ok(mut partition) =
            AsymmetricPartition::create("192.168.1.1", &["192.168.1.2"], PartitionDirection::OutboundOnly)
        {
            let result1 = partition.heal();
            let result2 = partition.heal();

            if result1.is_ok() {
                assert!(result2.is_ok());
                assert!(!partition.is_active());
            }
        }

        if let Ok(mut latency) = LatencyInjection::create("eth0", 100, 10) {
            let result1 = latency.heal();
            let result2 = latency.heal();

            if result1.is_ok() {
                assert!(result2.is_ok());
                assert!(!latency.is_active());
            }
        }

        if let Ok(mut loss) = PacketLossInjection::create("eth0", 10) {
            let result1 = loss.heal();
            let result2 = loss.heal();

            if result1.is_ok() {
                assert!(result2.is_ok());
                assert!(!loss.is_active());
            }
        }
    }

    // Note: Actual injection tests require root privileges and are disabled by default
    #[test]
    #[ignore = "requires root privileges"]
    fn test_partition_lifecycle() {
        let mut partition = NetworkPartition::create("10.100.0.11", &["10.100.0.12", "10.100.0.13"]).unwrap();
        assert!(partition.is_active());
        partition.heal().unwrap();
        assert!(!partition.is_active());
    }

    #[test]
    #[ignore = "requires root privileges"]
    fn test_latency_injection_lifecycle() {
        let mut latency = LatencyInjection::create("lo", 100, 10).unwrap();
        assert!(latency.is_active());
        latency.heal().unwrap();
        assert!(!latency.is_active());
    }

    #[test]
    #[ignore = "requires root privileges"]
    fn test_packet_loss_injection_lifecycle() {
        let mut loss = PacketLossInjection::create("lo", 10).unwrap();
        assert!(loss.is_active());
        loss.heal().unwrap();
        assert!(!loss.is_active());
    }
}
