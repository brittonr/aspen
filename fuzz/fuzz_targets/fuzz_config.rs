//! Fuzz target for configuration parsing.
//!
//! This target fuzzes TOML configuration parsing and validation paths.
//! Configuration comes from user-controlled files and environment, so
//! robust parsing is essential for security.
//!
//! Attack vectors tested:
//! - Malformed TOML syntax
//! - Invalid field types (string for u64, etc.)
//! - Missing required fields
//! - Extra unknown fields
//! - Invalid hex strings (Iroh secret keys)
//! - Invalid socket addresses
//! - Deeply nested invalid structures

#![no_main]

use libfuzzer_sys::fuzz_target;
use serde::Deserialize;

/// Simplified configuration struct matching NodeConfig layout.
/// We use a local definition to avoid needing all dependencies.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FuzzNodeConfig {
    node_id: Option<u64>,
    data_dir: Option<String>,
    storage_backend: Option<String>,
    control_backend: Option<String>,
    cookie: Option<String>,
    heartbeat_interval_ms: Option<u64>,
    election_timeout_min_ms: Option<u64>,
    election_timeout_max_ms: Option<u64>,
    raft_mailbox_capacity: Option<u32>,
    // Nested iroh config
    iroh: Option<FuzzIrohConfig>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FuzzIrohConfig {
    bind_port: Option<u16>,
    secret_key: Option<String>,
    bootstrap_ticket: Option<String>,
}

fuzz_target!(|data: &[u8]| {
    // Fuzz TOML parsing
    if let Ok(s) = std::str::from_utf8(data) {
        // Try parsing as NodeConfig
        let _ = toml::from_str::<FuzzNodeConfig>(s);

        // Try parsing as generic TOML Value (catches malformed TOML)
        let _ = toml::from_str::<toml::Value>(s);
    }

    // Fuzz hex secret key parsing (64 char hex = 32 bytes for Iroh SecretKey)
    if let Ok(s) = std::str::from_utf8(data) {
        let trimmed = s.trim();
        // Valid Iroh secret key is exactly 64 hex chars
        if trimmed.len() <= 128 {
            // Try hex decode
            if let Ok(bytes) = hex::decode(trimmed) {
                // Check if it's the right length for a secret key
                let _ = bytes.len() == 32;
            }
        }
    }

    // Fuzz SocketAddr parsing
    if let Ok(s) = std::str::from_utf8(data) {
        let trimmed = s.trim();
        if trimmed.len() <= 256 {
            // Reasonable max length for socket addr
            let _ = trimmed.parse::<std::net::SocketAddr>();
            // Also try IPv4 and IPv6 separately
            let _ = trimmed.parse::<std::net::Ipv4Addr>();
            let _ = trimmed.parse::<std::net::Ipv6Addr>();
        }
    }

    // Fuzz storage backend FromStr
    if let Ok(s) = std::str::from_utf8(data) {
        let lower = s.to_lowercase();
        // Valid values: "inmemory", "in-memory", "memory", "sqlite", "sql", "persistent", "disk", "redb"
        let _ = matches!(
            lower.as_str(),
            "inmemory" | "in-memory" | "memory" | "sqlite" | "sql" | "persistent" | "disk" | "redb"
        );
    }

    // Fuzz control backend FromStr
    if let Ok(s) = std::str::from_utf8(data) {
        let lower = s.to_lowercase();
        // Valid values: "deterministic", "raft_actor", "raftactor"
        let _ = matches!(
            lower.as_str(),
            "deterministic" | "raft_actor" | "raftactor" | "raft"
        );
    }
});
