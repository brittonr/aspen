let Contracts = import "../contracts.ncl" in
(import "../default.ncl") & {
  lifecycle_store_policy.required_non_claims = [],
} | Contracts.Policy
