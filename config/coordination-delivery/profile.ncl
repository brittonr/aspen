let Contracts = import "contracts.ncl" in
let VisibilityTimeoutTicks = 10 in
let MaximumAttempts = 2 in
let RetryDelayTicks = 5 in
let DeadLetterRetentionTicks = 20 in
let Capacity = 8 in
let MetadataByteLimit = 32 in
let StatusItemLimit = 8 in
let ServiceGeneration = 7 in
let PolicyRef = "blake3:05be03f3c3a2af25a8ba2f4f603b205b8105abdc15c61b4438518370b5e09d8a" in
let NonClaims = [
  "delivery-evidence-does-not-grant-authority",
  "delivery-evidence-does-not-prove-exactly-once-effects",
  "delivery-evidence-does-not-prove-payload-correctness",
  "delivery-evidence-does-not-prove-global-ordering",
  "delivery-evidence-does-not-prove-store-or-broker-correctness",
  "delivery-evidence-does-not-establish-release-eligibility",
]
in
{
  schema = "molten.coordination-delivery-profile.v1",
  policy = {
    schema = "molten.coordination-delivery-policy.v1",
    policy_id = "delivery-policy-v1",
    visibility_timeout_ticks = VisibilityTimeoutTicks,
    maximum_attempts = MaximumAttempts,
    retry_base_delay_ticks = RetryDelayTicks,
    retry_maximum_delay_ticks = RetryDelayTicks,
    retry_backoff = "fixed",
    ordering = "strict-fifo",
    dead_letter_queue_id = "queue:delivery-dlq",
    dead_letter_retention_ticks = DeadLetterRetentionTicks,
    ready_capacity = Capacity,
    in_flight_capacity = Capacity,
    retry_capacity = Capacity,
    dead_letter_capacity = Capacity,
    metadata_byte_limit = MetadataByteLimit,
    status_item_limit = StatusItemLimit,
    completion_authority_ref = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    expiry_authority_ref = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    redrive_authority_ref = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
    retention_authority_ref = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
    retryable_failure_classes = ["transient"],
    poison_failure_classes = ["poison"],
    poison_item_handling = "dead-letter",
    non_claims = NonClaims,
  },
  manifest = {
    schema = "molten.coordination-delivery-manifest.v1",
    extension_id = "coordination-delivery",
    service_id = "coordination-delivery-local",
    service_generation = ServiceGeneration,
    implementation_ref = "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
    time_profile_ref = "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    policy_ref = PolicyRef,
    port_bindings = {
      fabric_consistency = "blake3:1111111111111111111111111111111111111111111111111111111111111111",
      fabric_durable_state = "blake3:2222222222222222222222222222222222222222222222222222222222222222",
      fabric_logical_time = "blake3:3333333333333333333333333333333333333333333333333333333333333333",
      fabric_resource = "blake3:4444444444444444444444444444444444444444444444444444444444444444",
      fabric_observability = "blake3:5555555555555555555555555555555555555555555555555555555555555555",
    },
    non_claims = NonClaims,
  },
  time_domain = "logical",
  jitter = "none",
  base_fifo_unchanged = true,
  inline_payloads_allowed = false,
  claims_exactly_once = false,
  receipt_grants_authority = false,
  receipt_establishes_release_eligibility = false,
} | Contracts.Profile
