let Contracts = import "../contracts.ncl" in
(import "../default.ncl") & {
  task_marker_policy.markers | force = [{ id = "bad", token = "[sometimes]", kind = "literal" }],
} | Contracts.Policy
