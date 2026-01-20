# Development Configuration
#
# Fast iteration settings for local development.
# Uses in-memory storage and aggressive timeouts.

{
  node_id = 1,
  cookie = "dev-cluster",

  # In-memory for fast restarts during development
  storage_backend = 'inmemory,

  # Local discovery only
  iroh = {
    enable_gossip = true,
    enable_mdns = true,          # Local network discovery
    enable_dns_discovery = false,
    enable_pkarr = false,
    enable_raft_auth = false,    # Disabled for easier debugging
  },

  # Fast failover for development
  heartbeat_interval_ms = 100,
  election_timeout_min_ms = 300,
  election_timeout_max_ms = 600,

  # Optional: enable DNS server for testing
  dns_server = {
    enabled = true,
    listen_addr = "127.0.0.1:15353",
    default_ttl = 60,
  },
}
