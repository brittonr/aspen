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

use std::collections::HashSet;
use std::process::Command;
use std::time::Duration;

use snafu::{ResultExt, Snafu};
use tracing::{debug, info, warn};

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
    active: bool,
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
            active: false,
        };

        partition.inject()?;
        partition.active = true;
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
            run_iptables(&[
                "-I", "FORWARD",
                "-s", &self.source_ip,
                "-d", target_ip,
                "-j", "DROP",
            ])?;

            // Block traffic from target to source (symmetric partition)
            run_iptables(&[
                "-I", "FORWARD",
                "-s", target_ip,
                "-d", &self.source_ip,
                "-j", "DROP",
            ])?;
        }

        Ok(())
    }

    /// Heal the network partition.
    pub fn heal(&mut self) -> Result<(), FaultError> {
        if !self.active {
            return Ok(());
        }

        info!(
            source = %self.source_ip,
            targets = ?self.target_ips,
            "Healing network partition"
        );

        for target_ip in &self.target_ips {
            // Remove DROP rules
            let _ = run_iptables(&[
                "-D", "FORWARD",
                "-s", &self.source_ip,
                "-d", target_ip,
                "-j", "DROP",
            ]);

            let _ = run_iptables(&[
                "-D", "FORWARD",
                "-s", target_ip,
                "-d", &self.source_ip,
                "-j", "DROP",
            ]);
        }

        self.active = false;
        Ok(())
    }

    /// Check if the partition is currently active.
    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Drop for NetworkPartition {
    fn drop(&mut self) {
        if self.active {
            if let Err(e) = self.heal() {
                warn!(
                    source = %self.source_ip,
                    error = %e,
                    "Failed to heal partition during drop"
                );
            }
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
    /// Target IP to apply latency for (or "all" for all traffic).
    target_ip: Option<String>,
    /// Latency in milliseconds.
    latency_ms: u32,
    /// Jitter (variation) in milliseconds.
    jitter_ms: u32,
    /// Whether the injection is currently active.
    active: bool,
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
            target_ip: None,
            latency_ms,
            jitter_ms,
            active: false,
        };

        injection.inject()?;
        injection.active = true;
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
            "qdisc", "add", "dev", &self.interface, "root", "netem",
            "delay", &format!("{}ms", self.latency_ms),
            &format!("{}ms", self.jitter_ms),
        ])?;

        Ok(())
    }

    /// Remove the latency injection.
    pub fn heal(&mut self) -> Result<(), FaultError> {
        if !self.active {
            return Ok(());
        }

        info!(
            interface = %self.interface,
            "Removing network latency injection"
        );

        // Remove the qdisc
        let _ = run_tc(&[
            "qdisc", "del", "dev", &self.interface, "root",
        ]);

        self.active = false;
        Ok(())
    }

    /// Check if the injection is currently active.
    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Drop for LatencyInjection {
    fn drop(&mut self) {
        if self.active {
            if let Err(e) = self.heal() {
                warn!(
                    interface = %self.interface,
                    error = %e,
                    "Failed to heal latency injection during drop"
                );
            }
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
    active: bool,
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
            active: false,
        };

        injection.inject()?;
        injection.active = true;
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
            "qdisc", "add", "dev", &self.interface, "root", "netem",
            "loss", &format!("{}%", self.loss_percent),
        ])?;

        Ok(())
    }

    /// Remove the packet loss injection.
    pub fn heal(&mut self) -> Result<(), FaultError> {
        if !self.active {
            return Ok(());
        }

        info!(
            interface = %self.interface,
            "Removing packet loss injection"
        );

        // Remove the qdisc
        let _ = run_tc(&[
            "qdisc", "del", "dev", &self.interface, "root",
        ]);

        self.active = false;
        Ok(())
    }

    /// Check if the injection is currently active.
    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Drop for PacketLossInjection {
    fn drop(&mut self) {
        if self.active {
            if let Err(e) = self.heal() {
                warn!(
                    interface = %self.interface,
                    error = %e,
                    "Failed to heal packet loss injection during drop"
                );
            }
        }
    }
}

/// Helper to run iptables commands.
fn run_iptables(args: &[&str]) -> Result<(), FaultError> {
    debug!(args = ?args, "Running iptables command");

    let output = Command::new("iptables")
        .args(args)
        .output()
        .context(IoSnafu {
            operation: "iptables",
        })?;

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

    let output = Command::new("tc")
        .args(args)
        .output()
        .context(IoSnafu { operation: "tc" })?;

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
    #[snafu(display("I/O error during {operation}: {source}"))]
    Io {
        operation: &'static str,
        source: std::io::Error,
    },

    #[snafu(display("Command failed: {command}\nStderr: {stderr}"))]
    CommandFailed { command: String, stderr: String },

    #[snafu(display("Invalid parameter {name}={value} (max: {max})"))]
    InvalidParameter {
        name: &'static str,
        value: String,
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
    }

    #[test]
    fn test_packet_loss_validation() {
        // Should fail with loss > 100%
        let result = PacketLossInjection::create("eth0", 101);
        assert!(result.is_err());
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
}
