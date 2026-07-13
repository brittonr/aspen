let Contracts = import "contracts.ncl" in
let get = fun field fallback args => std.record.get_or field fallback args in
let DefaultBounds = {
  max_total_bytes = 1073741824,
  max_chunk_count = 1048576,
  max_chunk_bytes = 4194304,
  max_range_bytes = 67108864,
  max_concurrent_operations = 64,
  max_queued_bytes = 134217728,
  max_memory_bytes = 67108864,
  max_deadline_ticks = 600,
  max_retries = 3,
  max_events = 1048576,
  max_status_entries = 256,
} in
let profile = fun adapter_class adapter_capabilities args =>
let bound_args = get "bounds" {} args in
{
  class = adapter_class,
  capabilities = adapter_capabilities,
  bounds = {
    max_total_bytes = get "max_total_bytes" DefaultBounds.max_total_bytes bound_args,
    max_chunk_count = get "max_chunk_count" DefaultBounds.max_chunk_count bound_args,
    max_chunk_bytes = get "max_chunk_bytes" DefaultBounds.max_chunk_bytes bound_args,
    max_range_bytes = get "max_range_bytes" DefaultBounds.max_range_bytes bound_args,
    max_concurrent_operations = get "max_concurrent_operations" DefaultBounds.max_concurrent_operations bound_args,
    max_queued_bytes = get "max_queued_bytes" DefaultBounds.max_queued_bytes bound_args,
    max_memory_bytes = get "max_memory_bytes" DefaultBounds.max_memory_bytes bound_args,
    max_deadline_ticks = get "max_deadline_ticks" DefaultBounds.max_deadline_ticks bound_args,
    max_retries = get "max_retries" DefaultBounds.max_retries bound_args,
    max_events = get "max_events" DefaultBounds.max_events bound_args,
    max_status_entries = get "max_status_entries" DefaultBounds.max_status_entries bound_args,
  },
  supported_transforms = ["identity"],
  verify_before_exposure = get "verify_before_exposure" true args,
  persist_verified_partial_state = get "persist_verified_partial_state" true args,
  backend_hints_are_redacted = get "backend_hints_are_redacted" true args,
  backend_hints_are_identity = get "backend_hints_are_identity" false args,
  protection_grants_retention_authority = get "protection_grants_retention_authority" false args,
  pin_grants_read_authority = get "pin_grants_read_authority" false args,
  unprotect_grants_delete_authority = get "unprotect_grants_delete_authority" false args,
  live_network_required = get "live_network_required" false args,
  same_contract_as_live = get "same_contract_as_live" false args,
  non_claims = get "non_claims" Contracts.non_claims args,
} in
{
  make = fun args => {
    schema_id = Contracts.schema_id,
    schema_version = Contracts.schema_version,
    source_language = Contracts.source_language,
    profiles = {
      capability_local = profile "capability-local" [
        "streaming-put", "streaming-get", "verified-range", "availability",
        "import", "export", "protection", "durable-completion",
      ] (get "capability_local" {} args),
      redb_indexed = profile "redb-indexed" [
        "streaming-put", "streaming-get", "verified-range", "availability",
        "import", "export", "protection", "durable-completion",
      ] (get "redb_indexed" {} args),
      iroh_blobs = profile "iroh-blobs" [
        "streaming-get", "availability", "import", "export", "protection",
      ] ({ live_network_required = true } & get "iroh_blobs" {} args),
      deterministic_simulation = profile "deterministic-simulation" [
        "streaming-put", "streaming-get", "verified-range", "availability",
        "import", "export", "protection", "durable-completion",
      ] ({ same_contract_as_live = true } & get "deterministic_simulation" {} args),
    },
    non_claims = Contracts.non_claims,
  } | Contracts.Export,
}
