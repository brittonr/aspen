# Production Aspen Node Configuration
#
# Demonstrates Nickel's merge semantics for layered configuration.
# Use as a starting point for production deployments.

let base = {
  # Cluster authentication
  cookie = "change-this-secure-cookie",

  # Use persistent storage
  storage_backend = 'redb,

  # Production network settings
  iroh = {
    enable_gossip = true,
    enable_mdns = false,        # Disabled in production
    enable_dns_discovery = true,
    enable_pkarr = true,
    enable_raft_auth = true,    # Security enabled
    relay_mode = 'default,
  },

  # Blob replication for durability
  blobs = {
    enabled = true,
    auto_offload = true,
    offload_threshold_bytes = 1048576,  # 1 MB
    replication_factor = 3,
    min_replicas = 2,
  },

  # Conservative timing for production
  heartbeat_interval_ms = 1000,
  election_timeout_min_ms = 3000,
  election_timeout_max_ms = 6000,
} in

# Merge base config with node-specific settings
base & {
  node_id = 1,
  data_dir = "/var/lib/aspen/node-1",
  host = "0.0.0.0",
}
