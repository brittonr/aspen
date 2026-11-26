# r[impl molten.runtime_spine.preserves_boundary_profile.contract]
# r[impl molten.runtime_spine.preserves_boundary_profile.validation]
# r[impl molten.runtime_spine.preserves_boundary_profile.validation.core_coupling]
let Prelude = import "../nickel-contract-prelude.ncl" in
let Schema = "molten.preserves-boundary-profile.v1" in
let Families = ["node-control-envelope", "ticket", "workflow-bundle", "receipt", "evidence-envelope"] in
let RequiredNonClaim = "canonical-boundary-identity-only" in
# Each rule fails with its own deterministic diagnostic so a negative fixture can name the rule it breaks.
let rule = fun ok diagnostic => if ok then 'Ok else 'Error { message = diagnostic } in
let Family = Prelude.contract (Prelude.is_one_of Families) in
let SchemaLabel = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match "^molten\\.[A-Za-z0-9_.-]+\\.v1$" value
) in
let Blake3Ref = std.contract.from_validator (fun value =>
  rule (Prelude.is_blake3_ref value) "stale-blake3-ref: expected blake3: followed by 64 lowercase hex digits"
) in
let NonClaims = std.contract.from_validator (fun claims =>
  rule
    (
      std.is_array claims
      && std.array.all std.is_string claims
      && std.array.any (fun claim => std.string.is_match RequiredNonClaim claim) claims
    )
    "missing-boundary-non-claim: non_claims must include canonical-boundary-identity-only"
) in
let RowContract = {
  artifact_family | Family,
  schema_label | SchemaLabel,
  canonical_bytes_required | Bool,
  blake3_identity_field | Prelude.contracts.NonEmptyString,
  adapter_owner | Prelude.contracts.NonEmptyString,
  core_dto | Prelude.contracts.NonEmptyString,
  allowed_consumers | Prelude.contracts.NonEmptyUniqueStringArray | Array Prelude.contracts.NonEmptyString,
  non_claims | NonClaims,
  adapter_only | Bool,
} in
let MeasurementContract = {
  artifact_family | Family,
  schema_label | SchemaLabel,
  canonical_bytes | Bool,
  blake3_ref | Blake3Ref,
  consumer | Prelude.contracts.NonEmptyString,
  raw_preserves_core_coupling | Bool,
} in
let rows_cover_families = fun rows => Prelude.has_all (std.array.map (fun row => row.artifact_family) rows) Families in
let rows_are_unique = fun rows => Prelude.non_empty_distinct_by (fun row => row.artifact_family) rows in
let matching_rows = fun rows measurement =>
  std.array.filter (fun row => row.artifact_family == measurement.artifact_family && row.schema_label == measurement.schema_label) rows in
let first_issue = fun issues => if std.array.length issues == 0 then 'Ok else 'Error { message = std.array.first issues } in
let measurement_issues = fun rows measurement =>
  let candidates = matching_rows rows measurement in
  if std.array.length candidates == 0 then
    ["unknown-profile-surface: %{measurement.artifact_family}"]
  else
    let row = std.array.first candidates in
    (if row.canonical_bytes_required && !measurement.canonical_bytes then ["non-canonical-bytes: %{measurement.artifact_family}"] else [])
    @ (if std.array.elem measurement.consumer row.allowed_consumers then [] else ["unsupported-consumer: %{measurement.artifact_family} consumed by %{measurement.consumer}"])
    @ (if row.adapter_only && measurement.raw_preserves_core_coupling then ["raw-preserves-core-coupling: %{measurement.artifact_family} is adapter-only"] else []) in
let profile_issues = fun profile =>
  (if rows_cover_families profile.rows then [] else ["missing-profile-family: rows must cover every admitted family"])
  @ (if rows_are_unique profile.rows then [] else ["duplicate-profile-family: rows must name each family once"])
  @ std.array.flat_map (measurement_issues profile.rows) profile.measurements in
let ProfileContract = std.contract.Sequence [{
  schema | Prelude.contract (Prelude.is_exact_string Schema),
  rows | std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array RowContract],
  measurements | std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array MeasurementContract],
}, std.contract.from_validator (fun profile => first_issue (profile_issues profile))] in
{
  Families = Families,
  Row = RowContract,
  Measurement = MeasurementContract,
  Profile = ProfileContract,
}
