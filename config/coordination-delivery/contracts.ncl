let Prelude = import "../../docs/nickel-contract-prelude.ncl" in
let VisibilityTimeoutTicks = 10 in
let MaximumAttempts = 2 in
let RetryDelayTicks = 5 in
let DeadLetterRetentionTicks = 20 in
let Capacity = 8 in
let MetadataByteLimit = 32 in
let StatusItemLimit = 8 in
let ServiceGeneration = 7 in
let Blake3Ref =
  Prelude.contract (fun value =>
    std.is_string value && std.string.is_match "^blake3:[0-9a-f]{64}$" value
  )
in
let Policy = {
  schema | Prelude.contract (Prelude.is_exact_string "molten.coordination-delivery-policy.v1"),
  policy_id | Prelude.contracts.NonEmptyString,
  visibility_timeout_ticks | Number,
  maximum_attempts | Number,
  retry_base_delay_ticks | Number,
  retry_maximum_delay_ticks | Number,
  retry_backoff | Prelude.contract (Prelude.is_exact_string "fixed"),
  ordering | Prelude.contract (Prelude.is_exact_string "strict-fifo"),
  dead_letter_queue_id | Prelude.contracts.NonEmptyString,
  dead_letter_retention_ticks | Number,
  ready_capacity | Number,
  in_flight_capacity | Number,
  retry_capacity | Number,
  dead_letter_capacity | Number,
  metadata_byte_limit | Number,
  status_item_limit | Number,
  completion_authority_ref | Blake3Ref,
  expiry_authority_ref | Blake3Ref,
  redrive_authority_ref | Blake3Ref,
  retention_authority_ref | Blake3Ref,
  retryable_failure_classes | Array Prelude.contracts.NonEmptyString,
  poison_failure_classes | Array Prelude.contracts.NonEmptyString,
  poison_item_handling | Prelude.contract (Prelude.is_exact_string "dead-letter"),
  non_claims | Array Prelude.contracts.NonEmptyString,
}
in
let PortBindings = {
  fabric_consistency | Blake3Ref,
  fabric_durable_state | Blake3Ref,
  fabric_logical_time | Blake3Ref,
  fabric_resource | Blake3Ref,
  fabric_observability | Blake3Ref,
}
in
let Manifest = {
  schema | Prelude.contract (Prelude.is_exact_string "molten.coordination-delivery-manifest.v1"),
  extension_id | Prelude.contracts.NonEmptyString,
  service_id | Prelude.contracts.NonEmptyString,
  service_generation | Number,
  implementation_ref | Blake3Ref,
  time_profile_ref | Blake3Ref,
  policy_ref | Blake3Ref,
  port_bindings | PortBindings,
  non_claims | Array Prelude.contracts.NonEmptyString,
}
in
let ProfileShape = {
  schema | Prelude.contract (Prelude.is_exact_string "molten.coordination-delivery-profile.v1"),
  policy | Policy,
  manifest | Manifest,
  time_domain | Prelude.contract (Prelude.is_exact_string "logical"),
  jitter | Prelude.contract (Prelude.is_exact_string "none"),
  base_fifo_unchanged | Bool,
  inline_payloads_allowed | Bool,
  claims_exactly_once | Bool,
  receipt_grants_authority | Bool,
  receipt_establishes_release_eligibility | Bool,
}
in
let RequiredNonClaims = [
  "delivery-evidence-does-not-grant-authority",
  "delivery-evidence-does-not-prove-exactly-once-effects",
  "delivery-evidence-does-not-prove-payload-correctness",
  "delivery-evidence-does-not-prove-global-ordering",
  "delivery-evidence-does-not-prove-store-or-broker-correctness",
  "delivery-evidence-does-not-establish-release-eligibility",
]
in
let invariants = fun profile =>
  profile.policy.visibility_timeout_ticks == VisibilityTimeoutTicks
  && profile.policy.maximum_attempts == MaximumAttempts
  && profile.policy.retry_base_delay_ticks == RetryDelayTicks
  && profile.policy.retry_maximum_delay_ticks == RetryDelayTicks
  && profile.policy.dead_letter_retention_ticks == DeadLetterRetentionTicks
  && profile.policy.ready_capacity == Capacity
  && profile.policy.in_flight_capacity == Capacity
  && profile.policy.retry_capacity == Capacity
  && profile.policy.dead_letter_capacity == Capacity
  && profile.policy.metadata_byte_limit == MetadataByteLimit
  && profile.policy.status_item_limit == StatusItemLimit
  && profile.policy.retryable_failure_classes == ["transient"]
  && profile.policy.poison_failure_classes == ["poison"]
  && profile.policy.non_claims == RequiredNonClaims
  && profile.manifest.non_claims == RequiredNonClaims
  && profile.manifest.service_generation == ServiceGeneration
  && profile.manifest.policy_ref == "blake3:05be03f3c3a2af25a8ba2f4f603b205b8105abdc15c61b4438518370b5e09d8a"
  && profile.time_domain == "logical"
  && profile.jitter == "none"
  && profile.base_fifo_unchanged
  && !profile.inline_payloads_allowed
  && !profile.claims_exactly_once
  && !profile.receipt_grants_authority
  && !profile.receipt_establishes_release_eligibility
in
{
  Profile = std.contract.Sequence [ProfileShape, Prelude.contract invariants],
  Policy = Policy,
  Manifest = Manifest,
  PortBindings = PortBindings,
}
