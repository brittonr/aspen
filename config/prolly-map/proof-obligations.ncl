let Contracts = import "proof-contracts.ncl" in
let obligation = fun obligation_id obligation_statement obligation_assumptions obligation_status =>
  {
    id = obligation_id,
    target_owner = "trellis",
    statement = obligation_statement,
    assumptions = obligation_assumptions,
    status = obligation_status,
    proves_database_correctness = false,
    proves_collision_impossibility = false,
  }
in
{
  schema = "molten.prolly-map-proof-obligations.v1",
  trellis_revision = "0bf65150d4c75da5887d5cc53392c3da6b94b9d2",
  obligations = [
    obligation
      "prolly.sorted-unique"
      "canonical build preserves strict key order and uniqueness"
      ["input entries are admitted canonical bytes"]
      "model-checked",
    obligation
      "prolly.search-containment"
      "point and range reads return exactly entries within the requested key interval"
      ["all supplied node bytes pass canonical validation"]
      "model-checked",
    obligation
      "prolly.boundary-determinism"
      "equal profile key and encoded-size inputs yield equal split decisions"
      ["content identity framing is deterministic"]
      "model-checked",
    obligation
      "prolly.edit-preservation"
      "admitted edits preserve canonical ordering and the canonical equal-state root"
      ["the rebuild-first pilot receives a complete valid prior snapshot"]
      "model-checked",
    obligation
      "prolly.diff-soundness"
      "complete diff records exactly added removed and modified entries"
      ["equal BLAKE3 node identities are equal under collision resistance"]
      "model-checked",
    obligation
      "prolly.reachability"
      "complete supplied graph facts classify every node reachable from roots or pins"
      ["graph facts are complete and bind the supplied node identities"]
      "model-checked",
    obligation
      "prolly.formal-refinement"
      "future Trellis proofs refine production-linked Rust profile and node codecs"
      ["a reviewed production linkage is supplied"]
      "open-formal",
  ],
  formal_refinement_complete = false,
  claims_database_correctness = false,
  claims_collision_impossibility = false,
} | Contracts.Proof
