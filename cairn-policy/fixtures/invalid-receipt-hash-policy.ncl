let Contracts = import "../contracts.ncl" in
(import "../default.ncl") & {
  receipt_hash_policy.algorithm | force = "SHA256",
} | Contracts.Policy
