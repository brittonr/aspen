# Typed contract for Molten's source-scoped membership and placement profile.

let SchemaId = "molten.fabric.membership-placement-profile.v1" in
let SchemaVersion = 1 in
let SourceLanguage = "nickel" in
let RequiredNonClaims = [
  "does-not-prove-consensus",
  "does-not-provide-global-membership-truth",
  "does-not-prove-process-failure",
  "does-not-grant-capability-authority",
  "does-not-commit-placement",
  "connectivity-does-not-imply-eligibility",
  "does-not-prove-service-correctness",
] in
let ValidProviderKinds = ["static", "policy-managed", "consistency-backed", "deterministic-simulation"] in
let ValidAuthorityStrengths = ["observation-only", "operator-declared", "consistency-ordered", "externally-enforced"] in
let ValidFencingStrengths = ["process-local", "node-local-durable", "quorum-ordered", "externally-enforced"] in
let valid_ref = fun value =>
  std.is_string value
  && std.string.is_match "^blake3:[0-9a-f]{64}$" value in
let positive_integer = fun value =>
  std.is_number value
  && value > 0
  && value % 1 == 0 in
let valid_resources = fun value =>
  std.is_record value
  && positive_integer value.cpu_millis
  && positive_integer value.memory_bytes
  && positive_integer value.storage_bytes in
let valid_source = fun value =>
  std.is_record value
  && std.array.elem value.provider_kind ValidProviderKinds
  && std.array.elem value.authority_strength ValidAuthorityStrengths
  && valid_ref value.profile_ref
  && positive_integer value.max_view_age_ticks
  && (value.provider_kind != "deterministic-simulation"
      || value.authority_strength == "observation-only"
      || value.authority_strength == "operator-declared") in
let valid_fencing = fun value =>
  std.is_record value
  && std.array.elem value.enforcement ValidFencingStrengths
  && valid_ref value.profile_ref
  && valid_ref value.authority_ref
  && valid_ref value.effect_port_ref in
let valid_profile = fun value =>
  std.is_record value
  && value.schema_id == SchemaId
  && value.schema_version == SchemaVersion
  && value.source_language == SourceLanguage
  && valid_source value.source
  && valid_fencing value.fencing
  && valid_resources value.max_node_capacity
  && valid_resources value.max_role_request
  && value.max_role_request.cpu_millis <= value.max_node_capacity.cpu_millis
  && value.max_role_request.memory_bytes <= value.max_node_capacity.memory_bytes
  && value.max_role_request.storage_bytes <= value.max_node_capacity.storage_bytes
  && positive_integer value.max_replicas
  && positive_integer value.max_search_steps
  && value.non_claims == RequiredNonClaims in
let ExportContract = std.contract.from_predicate valid_profile in
{
  schema_id = SchemaId,
  schema_version = SchemaVersion,
  source_language = SourceLanguage,
  non_claims = RequiredNonClaims,
  Export = ExportContract,
}
