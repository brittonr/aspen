let Prelude = import "../../docs/nickel-contract-prelude.ncl" in
let Zero = 0 in
let Blake3Ref =
  Prelude.contract (fun value =>
    std.is_string value && std.string.is_match "^blake3:[0-9a-f]{64}$" value
  )
in
let SourceRevision =
  Prelude.contract (fun value =>
    std.is_string value && std.string.is_match "^[0-9a-f]{40}$" value
  )
in
let OptionalNonEmptyString =
  Prelude.contract (fun value =>
    value == null || (std.is_string value && value != "")
  )
in
let Status =
  Prelude.contract (
    Prelude.is_one_of [
      "compatible",
      "adapted",
      "intentional",
      "unsupported",
      "engine-gap",
    ]
  )
in
let LimitsContract = {
  max_adapted | Number,
  max_intentional | Number,
  max_unsupported | Number,
  max_engine_gap | Number,
}
in
let RowContract = {
  id | Prelude.contracts.NonEmptyString,
  source_contract | Prelude.contracts.NonEmptyString,
  status | Status,
  evidence_ref | Blake3Ref,
  fixture | Prelude.contracts.NonEmptyString,
  issue | OptionalNonEmptyString,
  explanation | Prelude.contracts.NonEmptyString,
}
in
let requires_issue = fun row =>
  (row.status != "unsupported" && row.status != "engine-gap") || row.issue != null
in
let count_status = fun rows status =>
  std.array.length (std.array.filter (fun row => row.status == status) rows)
in
let profile_invariants = fun profile =>
  Prelude.non_empty_distinct_by (fun row => row.id) profile.rows
  && std.array.all requires_issue profile.rows
  && count_status profile.rows "adapted" <= profile.limits.max_adapted
  && count_status profile.rows "intentional" <= profile.limits.max_intentional
  && count_status profile.rows "unsupported" <= profile.limits.max_unsupported
  && count_status profile.rows "engine-gap" <= profile.limits.max_engine_gap
  && profile.remotes_enabled == false
  && profile.vec1_enabled == false
  && profile.claims_molten_correctness == false
  && profile.claims_global_root_identity == false
  && profile.production_enabled == false
  && profile.limits.max_adapted >= Zero
  && profile.limits.max_intentional >= Zero
  && profile.limits.max_unsupported >= Zero
  && profile.limits.max_engine_gap >= Zero
in
let ProfileContract =
  std.contract.Sequence [
    {
      schema | Prelude.contract (Prelude.is_exact_string "molten.semantic-state-oracle-compatibility.v1"),
      repository | Prelude.contract (Prelude.is_exact_string "https://github.com/dolthub/doltlite"),
      source_revision | SourceRevision,
      backend_format | Prelude.contract (Prelude.is_exact_string "doltlite-chunk-store-v12"),
      remotes_enabled | Bool,
      vec1_enabled | Bool,
      claims_molten_correctness | Bool,
      claims_global_root_identity | Bool,
      production_enabled | Bool,
      limits | LimitsContract,
      rows | std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array RowContract],
    },
    Prelude.contract profile_invariants
  ]
in
{
  Profile = ProfileContract,
  Row = RowContract,
  Limits = LimitsContract,
}
