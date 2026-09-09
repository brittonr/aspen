let r = import "../readiness.ncl" in
{ snapshot | r.contracts.Snapshot = {
  scope = { change = "demo", policy_hash = "policy", source_hash = "source", evidence_hash = "evidence" },
  stages = [{ stage = 'completion, state = 'blocked, decision_hash = "decision" }],
}, action | r.contracts.Action = {
  id = "action", kind = 'evidence_refresh, causes = ["missing-result"], target = "check-id",
  prerequisites = ["current-subject"], authority = 'advisory,
  expected_observation = "current passing execution receipt", argv = [],
} }
