let Prelude = import "../../docs/nickel-contract-prelude.ncl" in
let ExactRevision = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match "^[0-9a-f]{40}$" value
) in
let Zero = 0 in
let ThreeReplicaCount = 3 in
let FiveReplicaCount = 5 in
let RequiredInvariantCount = 6 in
let RequiredNonClaimCount = 6 in
let PositiveBound = Prelude.contract (fun value =>
  std.is_number value && std.number.is_integer value && value > Zero
) in
let NodeCount = Prelude.contract (fun value =>
  std.is_number value
  && std.number.is_integer value
  && (value == ThreeReplicaCount || value == FiveReplicaCount)
) in
let Selection = Prelude.contract (Prelude.is_one_of ["pure-model", "deterministic-simulation"]) in
let ClaimProfile = Prelude.contract (Prelude.is_exact_string "pure-model-only") in
let Ordering = {
  receive_order_preserved | Bool,
  proposal_order_preserved | Bool,
  execution_order_preserved | Prelude.contract (fun value => value == true),
  acknowledgement_waits_for_proposal_evidence | Bool,
} in
let Source = {
  paper | Prelude.contract (Prelude.is_exact_string "Jetpack: Consensus Made Generally Fast, OSDI 2026"),
  artifact_source | Prelude.contract (Prelude.is_exact_string "https://github.com/stonysystems/jetpack"),
  artifact_revision | ExactRevision,
  artifact_license | Prelude.contract (Prelude.is_exact_string "MIT"),
} in
let Bounds = {
  commands | PositiveBound,
  keys | PositiveBound,
  views | PositiveBound,
  steps | PositiveBound,
} in
let ProfileContract = std.contract.Sequence [{
  schema | Prelude.contract (Prelude.is_exact_string "molten.consensus.fastpath-model-profile.v1"),
  profile_id | Prelude.contracts.StableId,
  source | Source,
  base_model_ref | Prelude.contracts.StableId,
  conflict_contract_ref | Prelude.contracts.StableId,
  fault_model | Prelude.contract (Prelude.is_exact_string "crash-fault"),
  node_count | NodeCount,
  active_proposers | Prelude.contracts.NonEmptyUniqueStringArray,
  bounds | Bounds,
  base_ordering | Ordering,
  selection | Selection,
  claim_profile | ClaimProfile,
  invariants | Prelude.contracts.NonEmptyUniqueStringArray,
  non_claims | Prelude.contracts.NonEmptyUniqueStringArray,
}, Prelude.contract (fun profile =>
  profile.base_ordering.proposal_order_preserved
  && (
    profile.base_ordering.receive_order_preserved
    || profile.base_ordering.acknowledgement_waits_for_proposal_evidence
  )
  && std.array.length profile.invariants == RequiredInvariantCount
  && std.array.length profile.non_claims >= RequiredNonClaimCount
)] in
{
  Profile = ProfileContract,
}
