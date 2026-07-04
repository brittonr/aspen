let Contracts = import "../contracts.ncl" in
(import "../default.ncl") & {
  task_marker_policy.markers | force = [
    { id = "parallel", token = "[parallel]", kind = "literal" },
    { id = "parallel", token = "[serial]", kind = "literal" },
  ],
} | Contracts.Policy
