# r[impl molten.runtime_spine.preserves_boundary_profile.contract]
let Prelude = import "../nickel-contract-prelude.ncl" in
let Schema = "molten.preserves-boundary-profile.v1" in
let Families = ["node-control-envelope", "ticket", "workflow-bundle", "receipt", "evidence-envelope"] in
let RequiredNonClaim = "canonical-boundary-identity-only" in
let Family = Prelude.contract (Prelude.is_one_of Families) in
let SchemaLabel = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match "^molten\\.[A-Za-z0-9_.-]+\\.v1$" value
) in
let NonClaims = Prelude.contract (fun claims =>
  std.is_array claims
  && std.array.all std.is_string claims
  && std.array.any (fun claim => std.string.is_match RequiredNonClaim claim) claims
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
  blake3_ref | Prelude.contracts.Blake3Ref,
  consumer | Prelude.contracts.NonEmptyString,
  raw_preserves_core_coupling | Bool,
} in
let rows_cover_families = fun rows => Prelude.has_all (std.array.map (fun row => row.artifact_family) rows) Families in
let rows_are_unique = fun rows => Prelude.non_empty_distinct_by (fun row => row.artifact_family) rows in
let measurement_allowed_by_row = fun measurement row =>
  row.artifact_family == measurement.artifact_family
  && row.schema_label == measurement.schema_label
  && (!row.canonical_bytes_required || measurement.canonical_bytes)
  && std.array.elem measurement.consumer row.allowed_consumers
  && (!row.adapter_only || !measurement.raw_preserves_core_coupling) in
let measurement_is_valid = fun rows measurement =>
  std.array.any (fun row => measurement_allowed_by_row measurement row) rows in
let profile_is_valid = fun profile =>
  rows_cover_families profile.rows
  && rows_are_unique profile.rows
  && std.array.all (measurement_is_valid profile.rows) profile.measurements in
let ProfileContract = std.contract.Sequence [{
  schema | Prelude.contract (Prelude.is_exact_string Schema),
  rows | std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array RowContract],
  measurements | std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array MeasurementContract],
}, Prelude.contract profile_is_valid] in
{
  Families = Families,
  Row = RowContract,
  Measurement = MeasurementContract,
  Profile = ProfileContract,
}
