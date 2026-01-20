# Basic Aspen Node Configuration
#
# This is a minimal configuration for a single Aspen node.
# Use for development or testing.

{
  node_id = 1,
  cookie = "development-cluster",
  storage_backend = 'redb,

  iroh = {
    enable_gossip = true,
    enable_mdns = true,
  },
}
