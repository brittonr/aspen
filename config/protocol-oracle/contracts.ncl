let Prelude = import "../../docs/nickel-contract-prelude.ncl" in
let StableIdPattern = "^[a-z0-9][a-z0-9.-]{0,95}$" in
let MaximumRecords = 65536 in
let MaximumStateBytes = 1048576 in
let MaximumTotalBytes = 268435456 in
let MaximumParticipants = 64 in
let MaximumCounters = 64 in
let MaximumVirtualProgressHorizon = 1000000 in
let Services = ["transactional-key-value", "replicated-log", "distributed-scheduler"] in
let LogicalPositions = ["version", "next-offset", "transition-count"] in
let SafetyScopes = ["local", "pairwise", "cohort", "durability"] in
let PhysicalLayouts = ["semantic-only", "canonical-physical"] in
let Completeness = ["lossless", "sampled"] in
let NoveltyFields = [
  "service",
  "participant",
  "transition-class",
  "logical-position",
  "state",
  "durability",
  "progress",
]
in
let RequiredNonClaims = [
  "simulation results do not prove KVM, live, production, or release behavior",
  "oracle results hold only for the recorded world, scheduler, workload, faults, and projections",
  "work counters are deterministic algorithmic observations, not hardware performance",
  "an independent oracle can share a defect with the implementation it checks",
]
in
let is_stable_id = fun value =>
  std.is_string value && std.string.is_match StableIdPattern value
in
let StableId = Prelude.contract is_stable_id in
let Positive = Prelude.contract Prelude.is_positive_integer in
let UniqueOneOf = fun allowed =>
  Prelude.contract (fun values =>
    Prelude.array_of (Prelude.is_one_of allowed) values
    && Prelude.distinct_strings values
  )
in
let Service = {
  service | Prelude.contract (Prelude.is_one_of Services),
  logical_position | Prelude.contract (Prelude.is_one_of LogicalPositions),
  safety | UniqueOneOf SafetyScopes,
  physical_layout | Prelude.contract (Prelude.is_one_of PhysicalLayouts),
}
in
let Participant = {
  participant | StableId,
  service | Prelude.contract (Prelude.is_one_of Services),
  is_required | Bool,
}
in
let Counter = {
  name | StableId,
  resets_on_generation | Bool,
}
in
let ProfileShape = {
  schema | Prelude.contract (Prelude.is_exact_string "molten.protocol-oracle-profile.v1"),
  profile_id | StableId,
  projection_schema | Prelude.contract (Prelude.is_exact_string "molten.protocol-projection.v1"),
  oracle | { oracle_id | StableId, version | Positive },
  oracle_ref | Prelude.contracts.Blake3Ref,
  services | Array Service,
  participants | Array Participant,
  completeness | Prelude.contract (Prelude.is_one_of Completeness),
  liveness
    | {
      requires_readiness | Bool,
      requires_membership | Bool,
      disruptive_faults_block | Bool,
      requires_durability | Bool,
      virtual_progress_horizon | Positive,
    },
  novelty_fields | UniqueOneOf NoveltyFields,
  counters | Array Counter,
  bounds
    | {
      max_records | Positive,
      max_state_bytes | Positive,
      max_total_bytes | Positive,
      max_participants | Positive,
      max_counters | Positive,
    },
  non_claims | Prelude.contracts.NonEmptyUniqueStringArray,
}
in
let profile_invariants = fun profile =>
  let bounds = profile.bounds in
  let declared = std.array.map (fun entry => entry.service) profile.services in
  bounds.max_records <= MaximumRecords
  && bounds.max_state_bytes <= MaximumStateBytes
  && bounds.max_total_bytes <= MaximumTotalBytes
  && bounds.max_participants <= MaximumParticipants
  && bounds.max_counters <= MaximumCounters
  && bounds.max_state_bytes <= bounds.max_total_bytes
  && std.array.length profile.services > 0
  && Prelude.distinct_by (fun entry => entry.service) profile.services
  && std.array.length profile.participants > 0
  && std.array.length profile.participants <= bounds.max_participants
  && Prelude.distinct_by (fun entry => entry.participant) profile.participants
  && std.array.all (fun entry => std.array.elem entry.service declared) profile.participants
  && std.array.length profile.counters > 0
  && std.array.length profile.counters <= bounds.max_counters
  && Prelude.distinct_by (fun counter => counter.name) profile.counters
  && profile.liveness.virtual_progress_horizon <= MaximumVirtualProgressHorizon
  && Prelude.has_all profile.non_claims RequiredNonClaims
in
{
  Profile = std.contract.Sequence [ProfileShape, Prelude.contract profile_invariants],
  required_non_claims = RequiredNonClaims,
}
