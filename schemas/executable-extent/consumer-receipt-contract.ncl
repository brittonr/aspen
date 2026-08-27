let DigestCharacters = 64 in
let RevisionCharacters = 40 in
let IsDigest = fun value =>
  std.is_string value
  && std.string.length value == DigestCharacters
  && std.string.is_match "^[0-9a-f]+$" value
in
let IsRevision = fun value =>
  std.is_string value
  && std.string.length value == RevisionCharacters
  && std.string.is_match "^[0-9a-f]+$" value
in
let AllowedReceiptFields = [
  "schema",
  "receipt_identity_blake3",
  "bundle_identity_blake3",
  "producer_receipt_identity_blake3",
  "semantic_code_identity_blake3",
  "built_artifact_identity_blake3",
  "layout_identity_blake3",
  "runtime_cohort_identity_blake3",
  "policy_identity_blake3",
  "disposition",
  "denial",
  "mappings",
  "extents_remeasured",
  "detached_review_admitted",
  "executable_extent_repository",
  "executable_extent_revision",
  "mantle_producer_repository",
  "mantle_producer_revision",
  "non_claims",
]
in
let RequiredNonClaims = [
  "consumer receipt does not prove compiler correctness",
  "consumer receipt does not prove executable code semantics",
  "consumer receipt does not prove sandbox or host integrity",
  "consumer receipt does not grant storage or retention authority",
  "consumer receipt does not prove external authority freshness",
  "consumer receipt does not prove release eligibility",
]
in
let AllowedMappingFields = [
  "ordinal",
  "extent_identity_blake3",
  "mapping_identity_blake3",
  "mapped_state",
  "final_state",
]
in
let IsMapping = fun value =>
  std.is_record value
  && std.array.all (fun field => std.array.elem field AllowedMappingFields) (std.record.fields value)
  && std.array.length (std.record.fields value) == std.array.length AllowedMappingFields
  && value.ordinal == 0
  && IsDigest value.extent_identity_blake3
  && IsDigest value.mapping_identity_blake3
  && value.mapped_state == "executable-read-only"
  && value.final_state == "unmapped"
in
let IsReceipt = fun value =>
  std.is_record value
  && std.array.all (fun field => std.array.elem field AllowedReceiptFields) (std.record.fields value)
  && std.array.length (std.record.fields value) == std.array.length AllowedReceiptFields
  && value.schema == "molten-executable-extent-consumer-receipt-v1"
  && IsDigest value.receipt_identity_blake3
  && IsDigest value.bundle_identity_blake3
  && IsDigest value.producer_receipt_identity_blake3
  && IsDigest value.semantic_code_identity_blake3
  && IsDigest value.built_artifact_identity_blake3
  && IsDigest value.layout_identity_blake3
  && IsDigest value.runtime_cohort_identity_blake3
  && IsDigest value.policy_identity_blake3
  && std.array.elem value.disposition ["mapped-and-unmapped", "inert"]
  && (value.denial == null || std.is_string value.denial)
  && std.is_array value.mappings
  && std.array.length value.mappings <= 1
  && std.array.all IsMapping value.mappings
  && value.extents_remeasured == true
  && value.detached_review_admitted == true
  && value.executable_extent_repository == "rad://z37R1bP1kHcELs89RNbQRaqbCVKxB"
  && IsRevision value.executable_extent_revision
  && value.mantle_producer_repository == "rad://z3DJe8tEdQuXpzTkfqCYQq6ZUqqkb"
  && IsRevision value.mantle_producer_revision
  && std.is_array value.non_claims
  && std.array.length value.non_claims == std.array.length RequiredNonClaims
  && std.array.all (fun claim => std.array.elem claim value.non_claims) RequiredNonClaims
  && (
    (value.disposition == "mapped-and-unmapped" && value.denial == null && std.array.length value.mappings == 1)
    || (value.disposition == "inert" && std.is_string value.denial && std.array.length value.mappings == 0)
  )
in
{
  ConsumerReceiptPredicate = IsReceipt,
  ConsumerReceipt = std.contract.from_predicate IsReceipt,
}
