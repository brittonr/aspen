let Contracts = import "../contracts.ncl" in
(import "../default.ncl") & {
  exemptions | force = "not-an-exemption-array",
} | Contracts.Policy
