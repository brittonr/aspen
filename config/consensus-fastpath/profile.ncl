let Contracts = import "contracts.ncl" in
let SourceRevision = "c03e318ec355b11edd42aac56c68d0765f88d1d2" in
let ThreeReplicaCount = 3 in
let FiveReplicaCount = 5 in
let CommandBound = 8 in
let KeyBound = 4 in
let ViewBound = 4 in
let StepBound = 64 in
let Invariants = [
  "recoverability",
  "no-conflicting-predecessor",
  "committed-order-agreement",
  "execution-order-agreement",
  "linearizable-conflict-order",
  "at-most-once-application",
] in
let NonClaims = [
  "no-live-engine-proof",
  "no-transport-proof",
  "no-durability-proof",
  "no-performance-proof",
  "no-production-linearizability-proof",
  "no-release-readiness",
] in
let Common = {
  schema = "molten.consensus.fastpath-model-profile.v1",
  source = {
    paper = "Jetpack: Consensus Made Generally Fast, OSDI 2026",
    artifact_source = "https://github.com/stonysystems/jetpack",
    artifact_revision = SourceRevision,
    artifact_license = "MIT",
  },
  base_model_ref = "molten-in-process-raft-model-v1",
  conflict_contract_ref = "molten-kv-conflict-contract-v1",
  fault_model = "crash-fault",
  active_proposers = ["node-a"],
  bounds = {
    commands = CommandBound,
    keys = KeyBound,
    views = ViewBound,
    steps = StepBound,
  },
  base_ordering = {
    receive_order_preserved = true,
    proposal_order_preserved = true,
    execution_order_preserved = true,
    acknowledgement_waits_for_proposal_evidence = false,
  },
  selection = "pure-model",
  claim_profile = "pure-model-only",
  invariants = Invariants,
  non_claims = NonClaims,
} in
{
  three_replica | Contracts.Profile = Common & {
    profile_id = "three-replica-fastpath-hazards-v1",
    node_count = ThreeReplicaCount,
  },
  five_replica | Contracts.Profile = Common & {
    profile_id = "five-replica-fastpath-hazards-v1",
    node_count = FiveReplicaCount,
  },
}
