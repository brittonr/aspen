let Prelude = import "../nickel-contract-prelude.ncl" in
let Layers = ["core", "codec", "policy-evidence", "runtime", "adapter", "cli", "test", "integration", "public-api", "generated"] in
let Layer = Prelude.contract (Prelude.is_one_of Layers) in
let GlobPattern = Prelude.contract (fun value =>
  std.is_string value && std.string.length value > 0 && !std.string.is_match "(^|/)\\.\\.(/|$)" value
) in
let RuleContract = {
  id | Prelude.contracts.StableId,
  owning_layer | Layer,
  source_globs | std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array GlobPattern],
  denied_targets | Array GlobPattern | default = [],
  allowed_targets | Array GlobPattern | default = [],
  diagnostic | Prelude.contracts.NonEmptyString,
  exemption_class | Prelude.contracts.StableId,
} in
let policy_invariants = fun policy =>
  Prelude.non_empty_distinct_by (fun rule => rule.id) policy.rules
  && std.array.all (fun rule => std.array.length rule.denied_targets > 0 || std.array.length rule.allowed_targets > 0) policy.rules in
let PolicyContract = std.contract.Sequence [{
  schema | Prelude.contract (Prelude.is_exact_string "molten.dependency-boundary-policy.v1"),
  rules | std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array RuleContract],
  generated_exemptions | Array Prelude.contracts.StableId | default = [],
}, Prelude.contract policy_invariants] in
{
  Layers = Layers,
  Rule = RuleContract,
  Policy = PolicyContract,
}
