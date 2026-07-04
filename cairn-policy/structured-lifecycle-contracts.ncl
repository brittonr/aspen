# Nickel contracts for structured Cairn lifecycle source.

let NonEmptyString = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.length value > 0
) in

let StableId = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.is_match "^[A-Za-z0-9_.-]+$" value
) in

let Strength = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "MUST" || value == "SHALL" || value == "SHOULD" || value == "MAY")
) in

let CoverageClass = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "positive" || value == "negative" || value == "malformed-input" || value == "boundary" || value == "regression")
) in

let NonEmptyArray = std.contract.from_predicate (fun value =>
  std.is_array value && std.array.length value > 0
) in

{
  Scenario = {
    id | StableId,
    given | NonEmptyArray | Array NonEmptyString,
    "when" | NonEmptyArray | Array NonEmptyString,
    "then" | NonEmptyArray | Array NonEmptyString,
  },

  Requirement = {
    id | StableId,
    strength | Strength,
    statement | NonEmptyString,
    scenarios | NonEmptyArray | Array Scenario,
  },

  Spec = {
    id | StableId,
    purpose | NonEmptyString,
    requirements | NonEmptyArray | Array Requirement,
  },

  Task = {
    id | StableId,
    covers | NonEmptyArray | Array StableId,
  },

  Probe = {
    id | StableId,
    requirements | NonEmptyArray | Array StableId,
    coverage_class | CoverageClass,
    evidence_role | NonEmptyString,
  },

  Change = {
    id | StableId,
    tasks | Array Task | default = [],
    probes | Array Probe | default = [],
  },

  Bundle = {
    schema | NonEmptyString,
    format_metadata | Dyn | default = {},
    specs | NonEmptyArray | Array Spec,
    changes | Array Change | default = [],
  },
}
