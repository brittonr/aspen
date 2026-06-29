let Ref = String in
let Text = String in
let ResourceLimits = {
  max_queue_depth | Number,
  max_receipt_bytes | Number,
  max_store_bytes | Number,
  max_delivery_latency_ms | Number,
  max_recovery_time_ms | Number,
} in
let StateLayout = {
  state_root | Text,
  ledger_dir | Text,
  redb_dir | Text,
  chunk_dir | Text,
  identity_dir | Text,
  retention_dir | Text,
  inbox_dir | Text,
} in
let ProductionNodeProfile = {
  profile_name | Text,
  state_layout | StateLayout,
  required_adapters | Array Text,
  source_gate_inputs | Array Ref,
  resource_limits | ResourceLimits,
  redaction_settings | Array Text,
  live_transport_settings | Array Text,
  startup_expectations | Array Text,
  shutdown_expectations | Array Text,
} in
{
  profile | ProductionNodeProfile = {
    profile_name = "pilot-node",
    state_layout = {
      state_root = "/var/lib/molten",
      ledger_dir = "ledger",
      redb_dir = "redb",
      chunk_dir = "chunks",
      identity_dir = "identity",
      retention_dir = "retention",
      inbox_dir = "inbox",
    },
    required_adapters = [
      "node-control",
      "evidence-ledger",
      "chunk-store",
      "retention-index",
      "live-iroh",
    ],
    source_gate_inputs = [
      "blake3:0000000000000000000000000000000000000000000000000000000000000000",
    ],
    resource_limits = {
      max_queue_depth = 1024,
      max_receipt_bytes = 134217728,
      max_store_bytes = 10737418240,
      max_delivery_latency_ms = 5000,
      max_recovery_time_ms = 60000,
    },
    redaction_settings = [
      "deny-sensitive-export-by-default",
      "diagnostic-logs-auxiliary-only",
      "private-ref-reveal-requires-receipt",
    ],
    live_transport_settings = [
      "node-control-topic-only",
      "ticket-bound-peer-admission",
      "authority-grant-ledger-required",
    ],
    startup_expectations = [
      "profile-ref-bound-in-startup-receipt",
      "source-gate-current-before-run",
      "required-adapters-start-in-deterministic-order",
    ],
    shutdown_expectations = [
      "bounded-drain-before-stop",
      "shutdown-receipt-imported",
      "active-lock-removed-after-pass",
    ],
  },
}
