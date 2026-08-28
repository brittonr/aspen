let Prelude = import "../../docs/nickel-contract-prelude.ncl" in
let Zero = 0 in
let One = 1 in
let RequiredMutations = 9 in
let RequiredPhases = 8 in
let RequiredCases = 72 in
let RequiredSchedules = 8 in
let RequiredScheduleSteps = 64 in
let RequiredAdapters = 9 in
let MaximumAdapters = 16 in
let MaximumUnsupportedRows = 16 in
let MaximumRestarts = 16 in
let MaximumObservations = 72 in
let Blake3Ref =
  Prelude.contract (fun value =>
    std.is_string value && std.string.is_match "^blake3:[0-9a-f]{64}$" value
  )
in
let SourceRevision =
  Prelude.contract (fun value =>
    std.is_string value && std.string.is_match "^[0-9a-f]{40}$" value
  )
in
let PositiveInteger =
  Prelude.contract (fun value =>
    std.is_number value && std.number.is_integer value && value >= One
  )
in
let NonNegativeInteger =
  Prelude.contract (fun value =>
    std.is_number value && std.number.is_integer value && value >= Zero
  )
in
let Mutation =
  Prelude.contract (
    Prelude.is_one_of [
      "capture",
      "head",
      "promotion",
      "witness",
      "outbox",
      "replication",
      "import",
      "retention",
      "garbage-collection",
    ]
  )
in
let Owner =
  Prelude.contract (
    Prelude.is_one_of [
      "world-commit",
      "world-head",
      "world-promotion",
      "world-head-witness",
      "world-promotion-outbox",
      "world-distribution",
      "world-replay",
      "world-retention",
      "world-garbage-collection",
    ]
  )
in
let Phase =
  Prelude.contract (
    Prelude.is_one_of [
      "uninterrupted",
      "before-submit",
      "after-possible-submit",
      "after-durable-write",
      "before-response",
      "lost-response",
      "process-restart",
      "recovery-read-back",
    ]
  )
in
let RecoveryClass =
  Prelude.contract (
    Prelude.is_one_of [
      "already-complete",
      "safe-to-retry",
      "superseded",
      "conflict",
      "uncertain",
      "denied",
      "corrupt",
      "manual-review",
    ]
  )
in
let Interleaving =
  Prelude.contract (
    Prelude.is_one_of [
      "prepare",
      "current-fact-recheck",
      "before-linearization",
      "durable-read-back",
    ]
  )
in
let LimitsContract =
  std.contract.Sequence [
    {
      max_cases | PositiveInteger,
      max_schedules | PositiveInteger,
      max_schedule_steps | PositiveInteger,
      max_adapters | PositiveInteger,
      max_observations | PositiveInteger,
      max_unsupported_rows | PositiveInteger,
      max_restarts | PositiveInteger,
    },
    Prelude.contract (fun limits =>
      limits.max_cases == RequiredCases
      && limits.max_schedules == RequiredSchedules
      && limits.max_schedule_steps == RequiredScheduleSteps
      && limits.max_adapters == MaximumAdapters
      && limits.max_observations == MaximumObservations
      && limits.max_unsupported_rows == MaximumUnsupportedRows
      && limits.max_restarts == MaximumRestarts
    )
  ]
in
let AdapterContract = {
  adapter_id | Prelude.contracts.NonEmptyString,
  owner | Owner,
  profile | Prelude.contracts.NonEmptyString,
  implementation_ref | Blake3Ref,
  semantic_phase_map_ref | Blake3Ref,
}
in
let CaseContract = {
  case_id | Prelude.contracts.NonEmptyString,
  mutation | Mutation,
  operation_id | Blake3Ref,
  phase | Phase,
  adapter_id | Prelude.contracts.NonEmptyString,
  expected_generation | PositiveInteger,
  pre_state_ref | Blake3Ref,
  expected_decision | RecoveryClass,
}
in
let StepContract = {
  position | NonNegativeInteger,
  operation_id | Blake3Ref,
  mutation | Mutation,
  expected_generation | PositiveInteger,
  pre_state_ref | Blake3Ref,
  interleaving | Interleaving,
  node_id | Prelude.contracts.NonEmptyString,
  node_generation | PositiveInteger,
}
in
let ScheduleContract =
  std.contract.Sequence [
    {
      schedule_id | Prelude.contracts.NonEmptyString,
      mutation | Mutation,
      winner_source | Prelude.contract (Prelude.is_exact_string "declared-interleaving"),
      wall_clock_timing | Prelude.contract (fun value => value == false),
      steps | std.contract.Sequence [Prelude.contract (fun values => std.array.length values == RequiredPhases), Array StepContract],
    },
    Prelude.contract (fun schedule =>
      std.array.all (fun step => step.mutation == schedule.mutation) schedule.steps
      && Prelude.non_empty_distinct_by
        (fun step => step.operation_id)
        (
          std.array.filter (fun step => step.position == Zero) schedule.steps
        )
    )
  ]
in
let MutationNames = [
  "capture",
  "head",
  "promotion",
  "witness",
  "outbox",
  "replication",
  "import",
  "retention",
  "garbage-collection",
]
in
let PhaseNames = [
  "uninterrupted",
  "before-submit",
  "after-possible-submit",
  "after-durable-write",
  "before-response",
  "lost-response",
  "process-restart",
  "recovery-read-back",
]
in
let ConcurrentMutationNames = [
  "head",
  "promotion",
  "witness",
  "outbox",
  "import",
  "replication",
  "retention",
  "garbage-collection",
]
in
let has_case = fun cases mutation phase =>
  std.array.any (fun case => case.mutation == mutation && case.phase == phase) cases
in
let profile_invariants = fun profile =>
  std.array.length profile.adapters == RequiredAdapters
  && Prelude.non_empty_distinct_by (fun adapter => adapter.adapter_id) profile.adapters
  && Prelude.non_empty_distinct_by (fun adapter => adapter.owner) profile.adapters
  && std.array.length profile.cases == RequiredCases
  && Prelude.non_empty_distinct_by (fun case => case.case_id) profile.cases
  && std.array.all
    (fun mutation => std.array.all (fun phase => has_case profile.cases mutation phase) PhaseNames)
    MutationNames
  && std.array.length profile.schedules == RequiredSchedules
  && Prelude.non_empty_distinct_by (fun schedule => schedule.schedule_id) profile.schedules
  && std.array.all
    (fun mutation => std.array.any (fun schedule => schedule.mutation == mutation) profile.schedules)
    ConcurrentMutationNames
  && std.array.length (std.array.flatten (std.array.map (fun schedule => schedule.steps) profile.schedules)) == RequiredScheduleSteps
  && std.array.length profile.unexplained_numeric_thresholds == Zero
  && profile.claims_independent_witness == false
  && profile.claims_physical_power_loss == false
in
let ProfileContract =
  std.contract.Sequence [
    {
      schema | Prelude.contract (Prelude.is_exact_string "molten.world-fault-profile.v1"),
      profile_name | Prelude.contracts.NonEmptyString,
      source_revision | SourceRevision,
      inventory_ref | Blake3Ref,
      rust_profile_ref | Blake3Ref,
      limits | LimitsContract,
      adapters | std.contract.Sequence [Prelude.contract (fun values => std.array.length values == RequiredAdapters), Array AdapterContract],
      cases | std.contract.Sequence [Prelude.contract (fun values => std.array.length values == RequiredCases), Array CaseContract],
      schedules | std.contract.Sequence [Prelude.contract (fun values => std.array.length values == RequiredSchedules), Array ScheduleContract],
      unexplained_numeric_thresholds | Array Number,
      claims_independent_witness | Bool,
      claims_physical_power_loss | Bool,
    },
    Prelude.contract profile_invariants
  ]
in
{
  Profile = ProfileContract,
  Limits = LimitsContract,
  Adapter = AdapterContract,
  Case = CaseContract,
  Schedule = ScheduleContract,
  Step = StepContract,
}
