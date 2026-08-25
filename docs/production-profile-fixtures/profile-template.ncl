let Contracts = import "../production-profile-contracts.ncl" in
let Thresholds = Contracts.thresholds in
# r[impl molten.prod_ops.production_profile.candidate_fixture_non_claim]
# Deterministic conformance input only. This value is not release evidence.
let FixtureCandidateSourceRef = "blake3:d54663d8d304af6d5f486ead64a51c06d74e1ab6320f62b2c6e9864dfe5f33c6" in
let get = fun field fallback overrides => std.record.get_or field fallback overrides in
{
  make = fun overrides =>
    let CandidateSourceRef = get "candidate_source_ref" FixtureCandidateSourceRef overrides in
    {
      candidate_source_ref = CandidateSourceRef,
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
        source_gate_inputs = get "source_gate_inputs" [CandidateSourceRef] overrides,
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
