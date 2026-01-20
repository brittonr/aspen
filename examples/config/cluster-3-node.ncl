# 3-Node Cluster Configuration Template
#
# This demonstrates Nickel's programmable configuration features:
# - Functions for generating node configs
# - Merge semantics for shared base settings
# - Contracts for validation
#
# Generate individual node configs by importing and selecting a node.

# Shared cluster settings
let cluster = {
  cookie = "my-production-cluster",
  storage_backend = 'redb,

  iroh = {
    enable_gossip = true,
    enable_mdns = false,
    enable_dns_discovery = true,
    enable_pkarr = true,
    enable_raft_auth = true,
  },

  blobs = {
    enabled = true,
    replication_factor = 3,
    min_replicas = 2,
  },

  heartbeat_interval_ms = 500,
  election_timeout_min_ms = 1500,
  election_timeout_max_ms = 3000,
} in

# Function to create a node config
# Note: Use distinct parameter names (n, h, d) to avoid shadowing field names
let make_node = fun n h d =>
  cluster & {
    node_id = n,
    host = h,
    data_dir = d,
  }
in

# Define all cluster nodes
let nodes = {
  node1 = make_node 1 "10.0.0.1" "/var/lib/aspen/node-1",
  node2 = make_node 2 "10.0.0.2" "/var/lib/aspen/node-2",
  node3 = make_node 3 "10.0.0.3" "/var/lib/aspen/node-3",
} in

# Export all nodes - select one when evaluating
# Usage: nickel export --field node1 cluster-3-node.ncl
nodes
