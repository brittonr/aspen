let r = import "../readiness.ncl" in
{ id = "action", kind = 'evidence_refresh, causes = [], target = "check-id",
  prerequisites = [], authority = 'mutation_grant,
  expected_observation = "passed", argv = [] } | r.contracts.Action
