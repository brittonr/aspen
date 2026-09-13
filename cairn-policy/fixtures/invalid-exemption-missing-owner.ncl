let Contracts = import "../contracts.ncl" in
(import "../default.ncl") & {
  exemptions | force = [{
    scope = "fixtures/basic/.cairn/changes/demo/tasks.md",
    reason = "fixture rejects missing exemption ownership",
    expires = "2099-01-01",
  }],
} | Contracts.Policy
