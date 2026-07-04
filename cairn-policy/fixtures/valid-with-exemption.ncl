let Contracts = import "../contracts.ncl" in
(import "../default.ncl") & {
  exemptions | force = [{
    scope = "fixtures/basic/cairn/changes/demo/tasks.md",
    owner = "test",
    reason = "fixture demonstrates bounded exemptions",
    expires = "2099-01-01",
  }],
} | Contracts.Policy
