let Contracts = import "../contracts.ncl" in
let Expected = [{
  scope = "fixtures/basic/.cairn/changes/demo/tasks.md",
  owner = "test",
  reason = "fixture demonstrates bounded exemptions",
  expires = "2099-01-01",
}] | Array Contracts.Exemption in
let ExactReplacement = std.contract.from_predicate (fun value => value == Expected) in
let EmptyDefault = std.contract.from_predicate (fun value => value == []) in
{
  replacement | ExactReplacement = (import "valid-with-exemption.ncl").exemptions,
  runtime_default | EmptyDefault = (import "../default.ncl").exemptions,
}
