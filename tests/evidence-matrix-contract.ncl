# Reviewed contract for the checked-in testing-harness evidence matrix (`tests/evidence-matrix.ncl`).
# Sequential/repeated placeholder digests and one command aliased across rows are rejected here.

let Prelude = import "../docs/nickel-contract-prelude.ncl" in
let CoverageKinds = ["positive", "negative"] in
let EvidenceScopes = ["unit", "property", "cli", "integration", "vm", "dogfood", "exemption"] in
let CoverageKind = Prelude.contract (Prelude.is_one_of CoverageKinds) in
let EvidenceScope = Prelude.contract (Prelude.is_one_of EvidenceScopes) in
# A digest with 128 leading zero bits or one repeated nibble names no real bytes.
let is_real_ref = fun value =>
  Prelude.is_blake3_ref value
  && !(std.string.is_match "^blake3:0{32}" value)
  && std.string.replace (std.string.substring 7 8 value) "" (std.string.substring 7 71 value) != "" in
let RealRef = Prelude.contract is_real_ref in
let EntryContract = {
  requirement_id | Prelude.contracts.StableId,
  coverage_kind | CoverageKind,
  evidence_scope | EvidenceScope,
  target | Prelude.contracts.NonEmptyString,
  command | Prelude.contracts.NonEmptyString,
  artifact_refs | Array RealRef,
  receipt_ref | RealRef,
  caveats | Array Prelude.contracts.NonEmptyString | default = [],
} in
let ExemptionContract = {
  requirement_id | Prelude.contracts.StableId,
  reason_class | Prelude.contracts.StableId,
  evidence_ref | RealRef,
  scope | EvidenceScope,
  review_note | Prelude.contracts.NonEmptyString,
} in
{
  schema | Prelude.contract (Prelude.is_exact_string "molten.testing.evidence-matrix.v1"),
  entries
    | std.contract.Sequence [
      Prelude.contracts.NonEmptyArray,
      # One executed command is evidence for exactly one row.
      Prelude.contract (Prelude.distinct_by (fun entry => entry.command)),
      Array EntryContract
    ],
  exemptions | Array ExemptionContract | default = [],
}
