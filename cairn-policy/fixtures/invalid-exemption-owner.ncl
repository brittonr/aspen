let Contracts = import "../contracts.ncl" in
(import "../default.ncl") & {
  exemptions | force = [{
    scope = "fixtures/basic/.cairn/changes/demo/tasks.md",
    owner = "",
    reason = "fixture rejects an empty exemption owner",
    expires = "2099-01-01",
  }],
} | Contracts.Policy
