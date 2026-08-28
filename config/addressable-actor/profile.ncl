let Contracts = import "contracts.ncl" in
let ProfileVersion = 1 in
let IdleAfterTicks = 5 in
let MaximumDrainItems = 16 in
let NonClaims = [
  "actor-identity-does-not-grant-authority",
  "wake-success-does-not-prove-transport-delivery",
  "checkpoint-possession-does-not-prove-survival",
  "runtime-processes-streams-sessions-callbacks-and-deltas-do-not-survive",
  "unknown-external-effects-are-not-retried-automatically",
  "actor-receipts-do-not-authorize-mutation-activation-release-or-retry",
]
in
{
  schema = "molten.addressable-actor.profile.v1",
  profile_id = "addressable-actor-v1",
  profile_version = ProfileVersion,
  reference_source = {
    repository = "https://github.com/rivet-dev/actors",
    revision = "71f371ba4eab1234d8b6b6c419e6748cc6fc9911",
    license = "Apache-2.0",
    selected_concepts = [
      "keyed-addressability",
      "generation-fenced-runtime",
      "sleep-intent-and-rewake-separation",
      "persisted-state-and-scheduled-events",
      "runtime-and-durable-survival-separation",
    ],
  },
  port_profiles = {
    system_extension = "blake3:1111111111111111111111111111111111111111111111111111111111111111",
    placement = "blake3:2222222222222222222222222222222222222222222222222222222222222222",
    coordination_delivery = "blake3:3333333333333333333333333333333333333333333333333333333333333333",
    durable_state = "blake3:4444444444444444444444444444444444444444444444444444444444444444",
    logical_time = "blake3:5555555555555555555555555555555555555555555555555555555555555555",
    resources = "blake3:6666666666666666666666666666666666666666666666666666666666666666",
    supervision = "blake3:7777777777777777777777777777777777777777777777777777777777777777",
    authority = "blake3:8888888888888888888888888888888888888888888888888888888888888888",
    evidence = "blake3:9999999999999999999999999999999999999999999999999999999999999999",
  },
  idle_after_ticks = IdleAfterTicks,
  maximum_drain_items = MaximumDrainItems,
  survival = {
    schema = "molten.addressable-actor.survival-matrix.v1",
    profile_version = ProfileVersion,
    rules = [
      { class = "durable-state", disposition = "durable" },
      { class = "mailbox-entries", disposition = "durable" },
      { class = "completed-semantic-events", disposition = "durable" },
      { class = "checkpoints", disposition = "durable" },
      { class = "processes", disposition = "runtime-only" },
      { class = "streams", disposition = "runtime-only" },
      { class = "sessions", disposition = "runtime-only" },
      { class = "partial-callbacks", disposition = "unsupported" },
      { class = "in-flight-deltas", disposition = "unsupported" },
    ],
  },
  non_claims = NonClaims,
  logical_time_only = true,
  automatic_external_retry = false,
  receipt_grants_authority = false,
  claims_global_key_uniqueness = false,
  claims_runtime_survival = false,
  inline_payloads_allowed = false,
} | Contracts.Profile
