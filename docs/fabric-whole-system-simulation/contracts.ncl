# Typed contract for bounded whole-system simulation review profiles.

let SchemaId = "molten.fabric-simulation.review-profile.v1" in
let SchemaVersion = 1 in
let SourceLanguage = "nickel" in
let RequiredPorts = [
  "authority",
  "transport",
  "durable-state",
  "time",
  "scheduling",
  "membership",
  "placement",
  "consistency",
  "supervision",
  "policy",
  "resources",
  "simulation",
  "evidence",
] in
let RequiredNonClaims = [
  "does-not-prove-live-transport",
  "does-not-prove-live-disk",
  "does-not-prove-operating-system-timing",
  "does-not-prove-production-scale",
  "does-not-prove-production-readiness",
  "does-not-prove-external-product-compatibility",
  "does-not-prove-arbitrary-schedule-correctness",
] in
let positive_integer = fun value =>
  std.is_number value
  && value > 0
  && value % 1 == 0 in
let valid_bounds = fun value =>
  std.is_record value
  && positive_integer value.max_choices
  && positive_integer value.max_events
  && positive_integer value.max_virtual_ticks
  && positive_integer value.max_trace_bytes
  && positive_integer value.max_resource_units
  && positive_integer value.max_shrink_attempts in
let valid_profile = fun value =>
  std.is_record value
  && value.schema_id == SchemaId
  && value.schema_version == SchemaVersion
  && value.source_language == SourceLanguage
  && value.claim_profile == "deterministic-whole-system"
  && value.port_classes == RequiredPorts
  && std.array.length value.nodes > 0
  && std.array.length value.workload > 0
  && valid_bounds value.bounds
  && value.ambient_inputs == []
  && value.non_claims == RequiredNonClaims in
let ExportContract = std.contract.from_predicate valid_profile in
{
  schema_id = SchemaId,
  schema_version = SchemaVersion,
  source_language = SourceLanguage,
  port_classes = RequiredPorts,
  non_claims = RequiredNonClaims,
  Export = ExportContract,
}
