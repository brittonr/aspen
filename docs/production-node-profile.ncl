let Contracts = import "production-profile-contracts.ncl" in
let Thresholds = Contracts.thresholds in
{
  # r[impl molten.prod_ops.production_profile.candidate_input]
  candidate_source_ref | Contracts.contracts.CandidateSourceRef,
  schema_id = Contracts.schema_id,
  schema_version = Contracts.schema_version,
  source_language = Contracts.source_language,
  profile_identity = "pilot-node",
  profile = {
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
    source_gate_inputs = [candidate_source_ref],
    resource_limits = {
      max_queue_depth = Thresholds.max_queue_depth,
      max_receipt_bytes = Thresholds.max_receipt_bytes,
      max_store_bytes = Thresholds.max_store_bytes,
      max_delivery_latency_ms = Thresholds.max_delivery_latency_ms,
      max_recovery_time_ms = Thresholds.max_recovery_time_ms,
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
