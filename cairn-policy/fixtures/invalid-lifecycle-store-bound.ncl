let Contracts = import "../contracts.ncl" in
(import "../default.ncl") & {
  lifecycle_store_policy.bounds.max_store_id_bytes = 0,
} | Contracts.Policy
