let Prelude = import "../../docs/nickel-contract-prelude.ncl" in
let Zero = 0 in
let One = 1 in
let ExactRevision = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match "^[0-9a-f]{40}$" value
) in
let Blake3Ref = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match "^blake3:[0-9a-f]{64}$" value
) in
let PositiveInteger = Prelude.contract (fun value =>
  std.is_number value && std.number.is_integer value && value >= One
) in
let NonNegativeInteger = Prelude.contract (fun value =>
  std.is_number value && std.number.is_integer value && value >= Zero
) in
let Preparation = Prelude.contract (Prelude.is_one_of ["cold", "declared-warm"]) in
let Class = Prelude.contract (Prelude.is_one_of ["logical", "opaque-exact-snapshot"]) in
let Shape = Prelude.contract (Prelude.is_one_of ["synthetic", "downstream-shaped"]) in
let Operation = Prelude.contract (Prelude.is_one_of [
  "root-branch",
  "first-mutation",
  "repeated-mutation",
  "diff",
  "merge-plan",
  "capsule-export",
  "replication",
  "retention-plan",
  "snapshot-share",
]) in
let Metric = Prelude.contract (Prelude.is_one_of [
  "logical-bytes",
  "physical-bytes-written",
  "new-objects",
  "reused-objects",
  "copied-pages",
  "mapped-pages",
  "traversed-references",
  "compared-keys",
  "emitted-conflicts",
  "transferred-bytes",
  "retained-objects",
  "planned-deletions",
]) in
let Bounds = {
  max_operations | PositiveInteger,
  max_repetitions | PositiveInteger,
  max_logical_bytes | PositiveInteger,
  max_physical_bytes | PositiveInteger,
  max_objects | PositiveInteger,
  max_pages | PositiveInteger,
  max_references | PositiveInteger,
  max_keys | PositiveInteger,
  max_conflicts | NonNegativeInteger,
  max_duration_nanoseconds | PositiveInteger,
  max_peak_memory_bytes | PositiveInteger,
} in
let DatasetContract = {
  dataset_ref | Blake3Ref,
  source_revision | ExactRevision,
  shape | Shape,
  preparation | Preparation,
  logical_bytes | PositiveInteger,
  object_count | PositiveInteger,
  preexisting_objects | NonNegativeInteger,
  changed_objects | PositiveInteger,
  mutation_bytes | PositiveInteger,
  key_count | PositiveInteger,
  page_count | PositiveInteger,
  page_size_bytes | PositiveInteger,
} in
let Threshold = {
  name | Prelude.contracts.NonEmptyString,
  metric | Metric,
  maximum | NonNegativeInteger,
  operation | Prelude.contract (Prelude.is_one_of [
    "all",
    "root-branch",
    "first-mutation",
    "repeated-mutation",
    "diff",
    "merge-plan",
    "capsule-export",
    "replication",
    "retention-plan",
    "snapshot-share",
  ]),
} in
let ProfileContract = std.contract.Sequence [{
  schema | Prelude.contract (Prelude.is_exact_string "molten.world-benchmark-profile.v1"),
  profile_ref | Blake3Ref,
  source_revision | ExactRevision,
  dataset_ref | Blake3Ref,
  preparation | Preparation,
  class | Class,
  adapters | Prelude.contracts.NonEmptyUniqueStringArray | Array Prelude.contracts.NonEmptyString,
  operations | Prelude.contracts.NonEmptyArray | Array Operation,
  bounds | Bounds,
  repetitions | PositiveInteger,
  hardware_cohort | Prelude.contracts.NonEmptyString,
  thresholds | Prelude.contracts.NonEmptyArray | Array Threshold,
}, Prelude.contract (fun profile =>
  profile.repetitions <= profile.bounds.max_repetitions
  && std.array.length profile.operations <= profile.bounds.max_operations
  && (
    (profile.class == "logical" && !std.array.elem "snapshot-share" profile.operations)
    || (profile.class == "opaque-exact-snapshot" && profile.operations == ["snapshot-share"])
  )
)] in
let InputContract = std.contract.Sequence [{
  profile | ProfileContract,
  dataset | DatasetContract,
}, Prelude.contract (fun input =>
  input.profile.dataset_ref == input.dataset.dataset_ref
  && input.profile.source_revision == input.dataset.source_revision
  && input.profile.preparation == input.dataset.preparation
  && input.dataset.changed_objects <= input.dataset.object_count
  && input.dataset.preexisting_objects <= input.dataset.object_count
  && input.dataset.mutation_bytes <= input.dataset.logical_bytes
  && (input.profile.preparation != "cold" || input.dataset.preexisting_objects == Zero)
)] in
{
  Input = InputContract,
  Profile = ProfileContract,
  Dataset = DatasetContract,
}
