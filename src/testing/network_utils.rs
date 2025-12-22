//! Network utilities for VM-based integration testing.
//!
//! This module provides utilities for managing network configuration
//! during VM-based cluster tests, including:
//!
//! - TAP device creation and management
//! - Bridge configuration
//! - IP address assignment
//! - Network namespace isolation
//!
//! # Tiger Style
//!
//! - All network operations require root/sudo privileges
//! - Operations are idempotent (can be called multiple times safely)
//! - Cleanup is automatic via Drop trait implementations
//!
//! # Example
//!
//! ```ignore
//! use aspen::testing::network_utils::{NetworkBridge, TapDevice};
//!
//! // Create a bridge for the test cluster
//! let bridge = NetworkBridge::create("aspen-br0", "10.100.0.1/24")?;
//!
//! // Create TAP devices for each VM
//! let tap1 = TapDevice::create("aspen-1", &bridge)?;
//! let tap2 = TapDevice::create("aspen-2", &bridge)?;
//!
//! // Cleanup happens automatically when variables go out of scope
//! ```

use std::process::Command;

use snafu::{ResultExt, Snafu};
use tracing::{debug, info, warn};

/// Network bridge for connecting VMs.
#[derive(Debug)]
pub struct NetworkBridge {
    /// Bridge interface name.
    pub name: String,
    /// IP address with CIDR notation (e.g., "10.100.0.1/24").
    pub ip_cidr: String,
    /// Whether to cleanup the bridge on drop.
    cleanup_on_drop: bool,
}

impl NetworkBridge {
    /// Create a new network bridge.
    ///
    /// # Arguments
    ///
    /// * `name` - Bridge interface name (e.g., "aspen-br0").
    /// * `ip_cidr` - IP address with CIDR notation (e.g., "10.100.0.1/24").
    ///
    /// # Errors
    ///
    /// Returns an error if bridge creation fails (usually requires root).
    pub fn create(name: &str, ip_cidr: &str) -> Result<Self, NetworkError> {
        let bridge = Self {
            name: name.to_string(),
            ip_cidr: ip_cidr.to_string(),
            cleanup_on_drop: true,
        };

        bridge.setup()?;
        Ok(bridge)
    }

    /// Attach to an existing bridge without creating it.
    pub fn attach(name: &str, ip_cidr: &str) -> Self {
        Self {
            name: name.to_string(),
            ip_cidr: ip_cidr.to_string(),
            cleanup_on_drop: false,
        }
    }

    /// Check if the bridge exists.
    pub fn exists(&self) -> bool {
        Command::new("ip")
            .args(["link", "show", &self.name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Set up the bridge interface.
    fn setup(&self) -> Result<(), NetworkError> {
        // Check if bridge already exists
        if self.exists() {
            info!(bridge = %self.name, "Bridge already exists, skipping creation");
            return Ok(());
        }

        info!(bridge = %self.name, ip = %self.ip_cidr, "Creating network bridge");

        // Create bridge
        run_ip_command(&["link", "add", "name", &self.name, "type", "bridge"])?;

        // Assign IP address
        run_ip_command(&["addr", "add", &self.ip_cidr, "dev", &self.name])?;

        // Bring up the interface
        run_ip_command(&["link", "set", &self.name, "up"])?;

        // Enable IP forwarding (may require sysctl)
        if let Err(e) = enable_ip_forwarding() {
            warn!(error = %e, "Failed to enable IP forwarding (may require root)");
        }

        Ok(())
    }

    /// Tear down the bridge interface.
    pub fn teardown(&self) -> Result<(), NetworkError> {
        if !self.exists() {
            debug!(bridge = %self.name, "Bridge does not exist, skipping teardown");
            return Ok(());
        }

        info!(bridge = %self.name, "Removing network bridge");

        // Bring down the interface
        run_ip_command(&["link", "set", &self.name, "down"])?;

        // Delete the bridge
        run_ip_command(&["link", "delete", &self.name])?;

        Ok(())
    }
}

impl Drop for NetworkBridge {
    fn drop(&mut self) {
        if self.cleanup_on_drop
            && let Err(e) = self.teardown()
        {
            warn!(
                bridge = %self.name,
                error = %e,
                "Failed to cleanup bridge during drop"
            );
        }
    }
}

/// TAP device for VM networking.
#[derive(Debug)]
pub struct TapDevice {
    /// TAP device name.
    pub name: String,
    /// Bridge this TAP is attached to.
    pub bridge_name: String,
    /// Whether to cleanup the TAP on drop.
    cleanup_on_drop: bool,
}

impl TapDevice {
    /// Create a new TAP device and attach it to a bridge.
    ///
    /// # Arguments
    ///
    /// * `name` - TAP device name (e.g., "aspen-1").
    /// * `bridge` - Bridge to attach the TAP device to.
    ///
    /// # Errors
    ///
    /// Returns an error if TAP creation fails (usually requires root).
    pub fn create(name: &str, bridge: &NetworkBridge) -> Result<Self, NetworkError> {
        let tap = Self {
            name: name.to_string(),
            bridge_name: bridge.name.clone(),
            cleanup_on_drop: true,
        };

        tap.setup()?;
        Ok(tap)
    }

    /// Attach to an existing TAP device without creating it.
    pub fn attach(name: &str, bridge_name: &str) -> Self {
        Self {
            name: name.to_string(),
            bridge_name: bridge_name.to_string(),
            cleanup_on_drop: false,
        }
    }

    /// Check if the TAP device exists.
    pub fn exists(&self) -> bool {
        Command::new("ip")
            .args(["link", "show", &self.name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Set up the TAP device.
    fn setup(&self) -> Result<(), NetworkError> {
        // Check if TAP already exists
        if self.exists() {
            info!(tap = %self.name, "TAP device already exists, skipping creation");
            return Ok(());
        }

        info!(
            tap = %self.name,
            bridge = %self.bridge_name,
            "Creating TAP device"
        );

        // Create TAP device
        run_ip_command(&["tuntap", "add", "name", &self.name, "mode", "tap"])?;

        // Attach to bridge
        run_ip_command(&["link", "set", &self.name, "master", &self.bridge_name])?;

        // Bring up the interface
        run_ip_command(&["link", "set", &self.name, "up"])?;

        Ok(())
    }

    /// Tear down the TAP device.
    pub fn teardown(&self) -> Result<(), NetworkError> {
        if !self.exists() {
            debug!(tap = %self.name, "TAP device does not exist, skipping teardown");
            return Ok(());
        }

        info!(tap = %self.name, "Removing TAP device");

        // Bring down the interface
        run_ip_command(&["link", "set", &self.name, "down"])?;

        // Delete the TAP device
        run_ip_command(&["link", "delete", &self.name])?;

        Ok(())
    }
}

impl Drop for TapDevice {
    fn drop(&mut self) {
        if self.cleanup_on_drop
            && let Err(e) = self.teardown()
        {
            warn!(
                tap = %self.name,
                error = %e,
                "Failed to cleanup TAP device during drop"
            );
        }
    }
}

/// Run an ip command with the given arguments.
fn run_ip_command(args: &[&str]) -> Result<(), NetworkError> {
    debug!(args = ?args, "Running ip command");

    let output = Command::new("ip").args(args).output().context(IoSnafu {
        operation: "ip command",
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(NetworkError::CommandFailed {
            command: format!("ip {}", args.join(" ")),
            stderr: stderr.to_string(),
        });
    }

    Ok(())
}

/// Enable IP forwarding via sysctl.
fn enable_ip_forwarding() -> Result<(), NetworkError> {
    let output = Command::new("sysctl")
        .args(["-w", "net.ipv4.ip_forward=1"])
        .output()
        .context(IoSnafu {
            operation: "sysctl ip_forward",
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(NetworkError::CommandFailed {
            command: "sysctl -w net.ipv4.ip_forward=1".to_string(),
            stderr: stderr.to_string(),
        });
    }

    Ok(())
}

/// Network-related errors.
#[derive(Debug, Snafu)]
pub enum NetworkError {
    /// I/O error during network operation.
    #[snafu(display("I/O error during {operation}: {source}"))]
    Io {
        /// The network operation that failed.
        operation: &'static str,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// External network command (ip, brctl, etc.) failed.
    #[snafu(display("Command failed: {command}\nStderr: {stderr}"))]
    CommandFailed {
        /// The command that failed.
        command: String,
        /// Standard error output from the command.
        stderr: String,
    },

    /// The specified network bridge was not found.
    #[snafu(display("Bridge {name} not found"))]
    BridgeNotFound {
        /// The bridge name that was not found.
        name: String,
    },

    /// The specified TAP device was not found.
    #[snafu(display("TAP device {name} not found"))]
    TapNotFound {
        /// The TAP device name that was not found.
        name: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require root privileges and may fail in sandboxed environments
    // They are disabled by default and should be run manually with appropriate permissions

    #[test]
    #[ignore = "requires root privileges"]
    fn test_bridge_lifecycle() {
        let bridge = NetworkBridge::create("test-br0", "192.168.100.1/24").unwrap();
        assert!(bridge.exists());
        drop(bridge);
        // Bridge should be cleaned up
    }

    #[test]
    #[ignore = "requires root privileges"]
    fn test_tap_lifecycle() {
        let bridge = NetworkBridge::create("test-br0", "192.168.100.1/24").unwrap();
        let tap = TapDevice::create("test-tap0", &bridge).unwrap();
        assert!(tap.exists());
        drop(tap);
        drop(bridge);
        // Both should be cleaned up
    }
}
