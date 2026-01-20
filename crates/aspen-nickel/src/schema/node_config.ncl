# Aspen NodeConfig Schema
# Nickel contracts for configuration validation
#
# This schema defines the structure and validation rules for Aspen cluster
# node configuration. Contracts provide runtime type checking and helpful
# error messages when configuration is invalid.

# === Primitive Contracts ===

let NonZeroU64 = std.contract.from_predicate (fun v =>
  std.is_number v && v > 0 && v % 1 == 0
) in

let PositiveNumber = std.contract.from_predicate (fun v =>
  std.is_number v && v > 0
) in

let NonNegativeNumber = std.contract.from_predicate (fun v =>
  std.is_number v && v >= 0
) in

let Port = std.contract.from_predicate (fun v =>
  std.is_number v && v >= 0 && v <= 65535 && v % 1 == 0
) in

let HexString64 = std.contract.from_predicate (fun v =>
  std.is_string v && std.string.length v == 64
) in

# === Enum Contracts ===

let StorageBackend = [| 'inmemory, 'redb |] in

let ControlBackend = [| 'deterministic, 'raft |] in

let RelayMode = [| 'default, 'custom, 'disabled |] in

# === Nested Config Contracts ===

# Iroh P2P networking configuration
let IrohConfig = {
  secret_key | String | optional,
  enable_gossip | Bool | default = true,
  gossip_ticket | String | optional,
  enable_mdns | Bool | default = true,
  enable_dns_discovery | Bool | default = false,
  dns_discovery_url | String | optional,
  enable_pkarr | Bool | default = false,
  enable_pkarr_dht | Bool | default = true,
  enable_pkarr_relay | Bool | default = true,
  include_pkarr_direct_addresses | Bool | default = true,
  pkarr_republish_delay_secs | Number | default = 600,
  pkarr_relay_url | String | optional,
  relay_mode | RelayMode | default = 'default,
  relay_urls | Array String | default = [],
  enable_raft_auth | Bool | default = false,
} in

# iroh-docs configuration for real-time sync
let DocsConfig = {
  enabled | Bool | default = true,
  enable_background_sync | Bool | default = true,
  background_sync_interval_secs | Number | default = 60,
  in_memory | Bool | default = false,
  namespace_secret | String | optional,
  author_secret | String | optional,
} in

# iroh-blobs configuration for content storage
let BlobConfig = {
  enabled | Bool | default = true,
  auto_offload | Bool | default = true,
  offload_threshold_bytes | Number | default = 1048576,
  gc_interval_secs | Number | default = 60,
  gc_grace_period_secs | Number | default = 300,
  replication_factor | Number | default = 1,
  min_replicas | Number | default = 1,
  max_replicas | Number | default = 5,
  repair_interval_secs | Number | default = 60,
  repair_delay_secs | Number | default = 300,
  enable_quorum_writes | Bool | default = false,
  failure_domain_key | String | optional,
  enable_auto_replication | Bool | default = false,
} in

# Peer cluster synchronization configuration
let PeerSyncConfig = {
  enabled | Bool | default = false,
  default_priority | Number | default = 100,
  max_subscriptions | Number | default = 32,
  reconnect_interval_secs | Number | default = 30,
  max_reconnect_attempts | Number | default = 0,
} in

# Horizontal sharding configuration
let ShardingConfig = {
  enabled | Bool | default = false,
  num_shards | Number | default = 4,
  local_shards | Array Number | default = [],
} in

# Cross-cluster federation configuration
let FederationConfig = {
  enabled | Bool | default = false,
  cluster_name | String | default = "aspen-cluster",
  cluster_key | String | optional,
  cluster_key_path | String | optional,
  enable_dht_discovery | Bool | default = true,
  enable_gossip | Bool | default = true,
  trusted_clusters | Array String | default = [],
  announce_interval_secs | Number | default = 1800,
  max_peers | Number | default = 256,
} in

# DNS protocol server configuration
let DnsServerConfig = {
  enabled | Bool | default = false,
  bind_addr | String | default = "127.0.0.1:5353",
  zones | Array String | default = ["aspen.local"],
  upstreams | Array String | default = ["8.8.8.8:53", "8.8.4.4:53"],
  forwarding_enabled | Bool | default = true,
} in

# Global content discovery configuration
let ContentDiscoveryConfig = {
  enabled | Bool | default = false,
  announce_interval_secs | Number | default = 1800,
  lookup_timeout_secs | Number | default = 30,
  max_concurrent_lookups | Number | default = 10,
  bootstrap_nodes | Array String | default = [],
  enable_ipv6 | Bool | default = false,
} in

# Worker pool configuration for job execution
let WorkerConfig = {
  enabled | Bool | default = false,
  worker_count | Number | default = 4,
  poll_interval_secs | Number | default = 1,
  visibility_timeout_secs | Number | default = 30,
  max_retries | Number | default = 3,
  retry_delay_secs | Number | default = 5,
  tags | Array String | default = [],
  max_concurrent_jobs | Number | default = 10,
  job_timeout_secs | Number | default = 300,
  heartbeat_interval_secs | Number | default = 10,
  dead_letter_enabled | Bool | default = true,
  max_dead_letter_age_secs | Number | default = 604800,
  enable_vm_executor | Bool | default = false,
  enable_shell_worker | Bool | default = false,
  shell_allowed_commands | Array String | default = [],
} in

# Event hook system configuration
let HooksConfig = {
  enabled | Bool | default = false,
  max_handlers | Number | default = 100,
  execution_timeout_secs | Number | default = 30,
} in

# Write batching configuration
let BatchConfig = {
  max_batch_size | Number | default = 100,
  max_batch_delay_ms | Number | default = 10,
} in

# === Main NodeConfig Contract ===

let NodeConfig = {
  # Core identification
  node_id | Number,
  data_dir | String | optional,
  storage_backend | StorageBackend | default = 'redb,
  redb_path | String | optional,
  host | String | default = "127.0.0.1",
  cookie | String,

  # Raft consensus
  control_backend | ControlBackend | default = 'raft,
  heartbeat_interval_ms | Number | default = 500,
  election_timeout_min_ms | Number | default = 1500,
  election_timeout_max_ms | Number | default = 3000,

  # Iroh P2P networking
  iroh | IrohConfig | default = {},
  docs | DocsConfig | default = {},
  blobs | BlobConfig | default = {},

  # Cluster coordination
  peer_sync | PeerSyncConfig | default = {},
  federation | FederationConfig | default = {},
  sharding | ShardingConfig | default = {},
  peers | Array String | default = [],

  # Optional subsystems
  batch_config | BatchConfig | optional,
  dns_server | DnsServerConfig | default = {},
  content_discovery | ContentDiscoveryConfig | default = {},
  worker | WorkerConfig | default = {},
  hooks | HooksConfig | default = {},
} in

# Export all contracts
{
  NodeConfig,
  IrohConfig,
  DocsConfig,
  BlobConfig,
  PeerSyncConfig,
  ShardingConfig,
  FederationConfig,
  DnsServerConfig,
  ContentDiscoveryConfig,
  WorkerConfig,
  HooksConfig,
  BatchConfig,
  StorageBackend,
  ControlBackend,
  RelayMode,
}
