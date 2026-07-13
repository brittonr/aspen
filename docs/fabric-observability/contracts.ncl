# Typed contract for Molten's bounded observability and integrity profile.

let SchemaId = "molten.fabric.observability-profile.v1" in
let SchemaVersion = 1 in
let SourceLanguage = "nickel" in
let AdapterClasses = [
  "tracing",
  "prometheus",
  "opentelemetry",
  "durable-state-scan",
  "content-verification",
  "runtime-counter",
  "deterministic-simulation",
] in
let ClaimScopes = [
  "local-component",
  "adapter",
  "system-extension",
  "cluster",
  "operator",
  "release-production",
] in
let RequiredNonClaims = [
  "observation-does-not-grant-capability-authority",
  "finding-does-not-grant-repair-authority",
  "health-does-not-prove-service-correctness",
  "local-observation-does-not-prove-cluster-truth",
  "readiness-does-not-prove-release-eligibility",
  "redaction-does-not-prove-confidentiality",
  "scan-does-not-prove-completeness-beyond-plan",
] in
let positive_integer = fun value =>
  std.is_number value
  && value > 0
  && value % 1 == 0 in
let valid_bounds = fun value =>
  std.is_record value
  && positive_integer value.max_descriptors
  && positive_integer value.max_labels_per_sample
  && positive_integer value.max_label_name_bytes
  && positive_integer value.max_label_value_bytes
  && positive_integer value.max_series
  && positive_integer value.max_events
  && positive_integer value.max_event_detail_bytes
  && positive_integer value.max_queued_bytes
  && positive_integer value.max_snapshots
  && positive_integer value.max_scan_items
  && positive_integer value.max_findings
  && positive_integer value.max_diagnostics
  && positive_integer value.min_export_interval_ticks in
let valid_profile = fun value =>
  std.is_record value
  && value.schema_id == SchemaId
  && value.schema_version == SchemaVersion
  && value.source_language == SourceLanguage
  && value.adapter_classes == AdapterClasses
  && value.claim_scopes == ClaimScopes
  && valid_bounds value.bounds
  && value.integrity.read_only_by_default
  && value.integrity.separate_mutation_authority_required
  && value.redaction.deny_unclassified_sensitive_values
  && value.redaction.deny_credentials
  && value.redaction.deny_raw_tickets
  && value.redaction.deny_payloads
  && value.redaction.deny_private_paths
  && !value.scope.allow_implicit_promotion
  && !value.export.allow_unbounded_queue
  && value.export.terminal_failures_explicit
  && value.non_claims == RequiredNonClaims in
let ExportContract = std.contract.from_predicate valid_profile in
{
  schema_id = SchemaId,
  schema_version = SchemaVersion,
  source_language = SourceLanguage,
  adapter_classes = AdapterClasses,
  claim_scopes = ClaimScopes,
  non_claims = RequiredNonClaims,
  Export = ExportContract,
}
