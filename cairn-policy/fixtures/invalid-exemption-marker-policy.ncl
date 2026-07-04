let Contracts = import "../contracts.ncl" in
(import "../default.ncl") & {
  exemption_marker_policy.required_fields | force = ["reason", "owner"],
} | Contracts.Policy
