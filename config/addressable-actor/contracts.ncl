let Prelude = import "../../docs/nickel-contract-prelude.ncl" in
let ProfileVersion = 1 in
let IdleAfterTicks = 5 in
let MaximumDrainItems = 16 in
let Blake3Ref =
  Prelude.contract (fun value =>
    std.is_string value && std.string.is_match "^blake3:[0-9a-f]{64}$" value
  )
in
let ReferenceSource = {
  repository | Prelude.contract (Prelude.is_exact_string "https://github.com/rivet-dev/actors"),
  revision | Prelude.contract (Prelude.is_exact_string "71f371ba4eab1234d8b6b6c419e6748cc6fc9911"),
  license | Prelude.contract (Prelude.is_exact_string "Apache-2.0"),
  selected_concepts | Array Prelude.contracts.NonEmptyString,
}
in
let PortProfiles = {
  system_extension | Blake3Ref,
  placement | Blake3Ref,
  coordination_delivery | Blake3Ref,
  durable_state | Blake3Ref,
  logical_time | Blake3Ref,
  resources | Blake3Ref,
  supervision | Blake3Ref,
  authority | Blake3Ref,
  evidence | Blake3Ref,
}
in
let SurvivalRule = {
  class | Prelude.contracts.NonEmptyString,
  disposition | Prelude.contracts.NonEmptyString,
}
in
let SurvivalMatrix = {
  schema | Prelude.contract (Prelude.is_exact_string "molten.addressable-actor.survival-matrix.v1"),
  profile_version | Number,
  rules | Array SurvivalRule,
}
in
let ProfileShape = {
  schema | Prelude.contract (Prelude.is_exact_string "molten.addressable-actor.profile.v1"),
  profile_id | Prelude.contracts.NonEmptyString,
  profile_version | Number,
  reference_source | ReferenceSource,
  port_profiles | PortProfiles,
  idle_after_ticks | Number,
  maximum_drain_items | Number,
  survival | SurvivalMatrix,
  non_claims | Array Prelude.contracts.NonEmptyString,
  logical_time_only | Bool,
  automatic_external_retry | Bool,
  receipt_grants_authority | Bool,
  claims_global_key_uniqueness | Bool,
  claims_runtime_survival | Bool,
  inline_payloads_allowed | Bool,
}
in
let RequiredConcepts = [
  "keyed-addressability",
  "generation-fenced-runtime",
  "sleep-intent-and-rewake-separation",
  "persisted-state-and-scheduled-events",
  "runtime-and-durable-survival-separation",
]
in
let RequiredSurvival = [
  { class = "durable-state", disposition = "durable" },
  { class = "mailbox-entries", disposition = "durable" },
  { class = "completed-semantic-events", disposition = "durable" },
  { class = "checkpoints", disposition = "durable" },
  { class = "processes", disposition = "runtime-only" },
  { class = "streams", disposition = "runtime-only" },
  { class = "sessions", disposition = "runtime-only" },
  { class = "partial-callbacks", disposition = "unsupported" },
  { class = "in-flight-deltas", disposition = "unsupported" },
]
in
let RequiredNonClaims = [
  "actor-identity-does-not-grant-authority",
  "wake-success-does-not-prove-transport-delivery",
  "checkpoint-possession-does-not-prove-survival",
  "runtime-processes-streams-sessions-callbacks-and-deltas-do-not-survive",
  "unknown-external-effects-are-not-retried-automatically",
  "actor-receipts-do-not-authorize-mutation-activation-release-or-retry",
]
in
let invariants = fun profile =>
  profile.profile_version == ProfileVersion
  && profile.reference_source.selected_concepts == RequiredConcepts
  && profile.idle_after_ticks == IdleAfterTicks
  && profile.maximum_drain_items == MaximumDrainItems
  && profile.survival.profile_version == ProfileVersion
  && profile.survival.rules == RequiredSurvival
  && profile.non_claims == RequiredNonClaims
  && profile.logical_time_only
  && !profile.automatic_external_retry
  && !profile.receipt_grants_authority
  && !profile.claims_global_key_uniqueness
  && !profile.claims_runtime_survival
  && !profile.inline_payloads_allowed
in
{
  Profile = std.contract.Sequence [ProfileShape, Prelude.contract invariants],
  PortProfiles = PortProfiles,
  SurvivalMatrix = SurvivalMatrix,
}
