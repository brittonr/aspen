# Dogfooding Configuration for Aspen Self-Hosting
#
# This configuration enables all features needed to run Aspen's own
# CI/CD infrastructure: Forge (Git hosting), CI pipelines, and Nix cache.
#
# Usage:
#   1. Start a cluster with this config
#   2. Create the Aspen repository in Forge
#   3. Push Aspen source to the cluster
#   4. CI automatically triggers on pushes
#
# Note: watched_repos will be empty initially. After creating the repo,
# either:
#   - Set ASPEN_CI_WATCHED_REPOS environment variable
#   - Or manually call watch_repo via CLI (future feature)

let base = {
  # Cluster authentication - change for production
  cookie = "aspen-dogfood-cluster",

  # Use persistent storage for durability
  storage_backend = 'redb,

  # Network configuration
  iroh = {
    enable_gossip = true,
    enable_mdns = true,        # Local discovery for dev clusters
    enable_dns_discovery = false,
    enable_pkarr = false,
    enable_raft_auth = true,   # Security enabled
    relay_mode = 'default,
  },

  # Blob storage with full replication
  blobs = {
    enabled = true,
    auto_offload = true,
    offload_threshold_bytes = 1048576,  # 1 MB
    replication_factor = 3,
    min_replicas = 2,
  },

  # CI/CD configuration
  ci = {
    enabled = true,
    auto_trigger = true,
    max_concurrent_runs = 3,
    pipeline_timeout_secs = 7200,  # 2 hours for full builds
    watched_repos = [],            # Populated after repo creation
  },

  # Forge (Git hosting)
  forge = {
    enable_gossip = true,
  },

  # Nix binary cache gateway
  nix_cache = {
    enabled = true,
    priority = 20,             # Prefer over cache.nixos.org (priority 40)
    want_mass_query = true,
  },

  # Workers for executing CI jobs
  workers = {
    enabled = true,
    worker_count = 2,
    max_concurrent_jobs = 4,
    prefer_local = true,
  },

  # Reasonable timing for local clusters
  heartbeat_interval_ms = 500,
  election_timeout_min_ms = 1500,
  election_timeout_max_ms = 3000,
} in

# Function to create node-specific configs
# Use distinct parameter names to avoid field shadowing
let make_node = fun n h d =>
  base & {
    node_id = n,
    host = h,
    data_dir = d,
  }
in

# Define all cluster nodes
let nodes = {
  node1 = make_node 1 "127.0.0.1" "/tmp/aspen-dogfood/node-1",
  node2 = make_node 2 "127.0.0.1" "/tmp/aspen-dogfood/node-2",
  node3 = make_node 3 "127.0.0.1" "/tmp/aspen-dogfood/node-3",
} in

# Export all nodes - select one when evaluating
# Usage: nickel export --field node1 dogfood.ncl
nodes
