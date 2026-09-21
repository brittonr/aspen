# Typed contract for the marble object-store pilot dependency catalog.

let SchemaId = "molten.marble-store-pilot.catalog.v1" in
let RequiredNonClaims = [
  "spike-is-not-default-path",
  "object-id-is-not-content-identity",
  "measurement-is-not-proof",
  "recovery-claims-stop-at-recovered-batches",
] in
let Semver = fun value =>
  std.is_string value
  && std.string.is_match "^[0-9]+\\.[0-9]+\\.[0-9]+$" value in
let valid_dependency = fun dependency =>
  std.is_record dependency
  && std.is_string dependency.crate
  && Semver dependency.pinned_version
  && dependency.transport == "crates.io"
  && dependency.plane == "implementation"
  && std.is_string dependency.license
  && std.string.length dependency.license > 0
  && std.array.all (fun claim => std.elem claim RequiredNonClaims) dependency.consumed_claims
  && std.array.length dependency.consumed_claims == 0 in
let valid_catalog = fun value =>
  std.is_record value
  && value.schema_id == SchemaId
  && std.array.length value.dependencies == 2
  && std.array.all valid_dependency value.dependencies
  && std.array.all
    (fun dependency => dependency.crate == "marble" || dependency.crate == "art")
    value.dependencies
  && value.non_claims == RequiredNonClaims in
{
  schema_id = SchemaId,
  non_claims = RequiredNonClaims,
  Export = std.contract.from_predicate valid_catalog,
}
