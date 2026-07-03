let Contracts = import "../production-profile-contracts.ncl" in
let Thresholds = Contracts.thresholds in
let get = fun field default overrides => std.record.get_or field default overrides in
{
  make = fun overrides => {
    schema_id = get "schema_id" Contracts.schema_id overrides,
    schema_version = get "schema_version" Contracts.schema_version overrides,
    source_language = get "source_language" Contracts.source_language overrides,
    profile_identity = get "profile_identity" "pilot-node" overrides,
    profile = {
      profile_name = get "profile_name" "pilot-node" overrides,
      state_layout = get "state_layout" {
        state_root = "/var/lib/molten",
        ledger_dir = "ledger",
        redb_dir = "redb",
        chunk_dir = "chunks",
        identity_dir = "identity",
        retention_dir = "retention",
        inbox_dir = "inbox",
      } overrides,
      required_adapters = get "required_adapters" [
        "node-control",
        "evidence-ledger",
        "chunk-store",
        "retention-index",
        "live-iroh",
      ] overrides,
      source_gate_inputs = get "source_gate_inputs" [
        "blake3:0000000000000000000000000000000000000000000000000000000000000000",
      ] overrides,
      resource_limits = get "resource_limits" {
        max_queue_depth = Thresholds.max_queue_depth,
        max_receipt_bytes = Thresholds.max_receipt_bytes,
        max_store_bytes = Thresholds.max_store_bytes,
        max_delivery_latency_ms = Thresholds.max_delivery_latency_ms,
        max_recovery_time_ms = Thresholds.max_recovery_time_ms,
      } overrides,
      redaction_settings = get "redaction_settings" [
        "deny-sensitive-export-by-default",
        "diagnostic-logs-auxiliary-only",
        "private-ref-reveal-requires-receipt",
      ] overrides,
      live_transport_settings = get "live_transport_settings" [
        "node-control-topic-only",
        "ticket-bound-peer-admission",
        "authority-grant-ledger-required",
      ] overrides,
      startup_expectations = get "startup_expectations" [
        "profile-ref-bound-in-startup-receipt",
        "source-gate-current-before-run",
        "required-adapters-start-in-deterministic-order",
      ] overrides,
      shutdown_expectations = get "shutdown_expectations" [
        "bounded-drain-before-stop",
        "shutdown-receipt-imported",
        "active-lock-removed-after-pass",
      ] overrides,
    },
  } | Contracts.ProductionProfileExport,
}
