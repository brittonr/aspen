# Typed contract for Molten content-store adapter profiles.

let SchemaId = "molten.content-store-adapter.profile-set.v1" in
let SchemaVersion = 1 in
let SourceLanguage = "nickel" in
let RequiredNonClaims = [
  "backend-hint-is-not-identity",
  "transport-is-not-import-authority",
  "protection-is-not-retention-authority",
  "pin-is-not-read-authority",
  "unprotect-is-not-delete-authority",
  "availability-is-not-execution-authority",
  "verification-is-not-provenance",
] in
let positive_integer = fun value =>
  std.is_number value
  && value > 0
  && value % 1 == 0 in
let valid_bounds = fun value =>
  std.is_record value
  && positive_integer value.max_total_bytes
  && positive_integer value.max_chunk_count
  && positive_integer value.max_chunk_bytes
  && positive_integer value.max_range_bytes
  && positive_integer value.max_concurrent_operations
  && positive_integer value.max_queued_bytes
  && positive_integer value.max_memory_bytes
  && positive_integer value.max_deadline_ticks
  && positive_integer value.max_events
  && positive_integer value.max_status_entries
  && std.is_number value.max_retries
  && value.max_retries >= 0
  && value.max_retries % 1 == 0 in
let valid_profile = fun profile =>
  std.is_record profile
  && std.is_string profile.class
  && std.is_array profile.capabilities
  && std.array.length profile.capabilities > 0
  && valid_bounds profile.bounds
  && profile.supported_transforms == ["identity"]
  && profile.verify_before_exposure
  && profile.persist_verified_partial_state
  && profile.backend_hints_are_redacted
  && !profile.backend_hints_are_identity
  && !profile.protection_grants_retention_authority
  && !profile.pin_grants_read_authority
  && !profile.unprotect_grants_delete_authority
  && profile.non_claims == RequiredNonClaims in
let valid_profile_set = fun value =>
  std.is_record value
  && value.schema_id == SchemaId
  && value.schema_version == SchemaVersion
  && value.source_language == SourceLanguage
  && valid_profile value.profiles.capability_local
  && value.profiles.capability_local.class == "capability-local"
  && valid_profile value.profiles.redb_indexed
  && value.profiles.redb_indexed.class == "redb-indexed"
  && valid_profile value.profiles.iroh_blobs
  && value.profiles.iroh_blobs.class == "iroh-blobs"
  && value.profiles.iroh_blobs.live_network_required
  && valid_profile value.profiles.deterministic_simulation
  && value.profiles.deterministic_simulation.class == "deterministic-simulation"
  && value.profiles.deterministic_simulation.same_contract_as_live
  && value.non_claims == RequiredNonClaims in
{
  schema_id = SchemaId,
  schema_version = SchemaVersion,
  source_language = SourceLanguage,
  non_claims = RequiredNonClaims,
  Export = std.contract.from_predicate valid_profile_set,
}
