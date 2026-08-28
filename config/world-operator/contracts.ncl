let Prelude = import "../../docs/nickel-contract-prelude.ncl" in
let Zero = 0 in
let One = 1 in
let MaximumOperations = 32 in
let MaximumDependencies = 32 in
let MaximumReceiptLinks = 64 in
let MaximumCanonicalBytes = 262144 in
let Blake3Ref = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match "^blake3:[0-9a-f]{64}$" value
) in
let PositiveInteger = Prelude.contract (fun value =>
  std.is_number value && std.number.is_integer value && value >= One
) in
let NonNegativeInteger = Prelude.contract (fun value =>
  std.is_number value && std.number.is_integer value && value >= Zero
) in
let OperationKind = Prelude.contract (Prelude.is_one_of [
  "inspect",
  "checkpoint",
  "branch",
  "run",
  "diff",
  "conflicts",
  "replay",
  "simulate",
  "verify",
  "promote",
  "export",
  "import",
  "gc-plan",
]) in
let ProfileKind = Prelude.contract (Prelude.is_one_of [
  "logical",
  "opaque",
  "witnessed-head",
  "executable-extent",
]) in
let ProfileStatus = Prelude.contract (Prelude.is_one_of [
  "admitted",
  "blocked",
  "unsupported",
  "unavailable",
]) in
let LimitsContract = std.contract.Sequence [{
  limits_ref | Blake3Ref,
  max_operations | PositiveInteger,
  max_dependencies_per_operation | PositiveInteger,
  max_receipt_links | PositiveInteger,
  max_canonical_bytes | PositiveInteger,
}, Prelude.contract (fun limits =>
  limits.max_operations <= MaximumOperations
  && limits.max_dependencies_per_operation <= MaximumDependencies
  && limits.max_receipt_links >= limits.max_operations
  && limits.max_receipt_links <= MaximumReceiptLinks
  && limits.max_canonical_bytes <= MaximumCanonicalBytes
)] in
let ProfileContract = {
  profile_ref | Blake3Ref,
  kind | ProfileKind,
  status | ProfileStatus,
  status_ref | Blake3Ref,
} in
let ObservationContract = {
  kind | Prelude.contracts.NonEmptyString,
  observation_ref | Blake3Ref,
  subject_ref | Blake3Ref,
  admitted | Bool,
} in
let OperationContract = {
  operation_id | Blake3Ref,
  kind | OperationKind,
  subject_ref | Blake3Ref,
  profile_ref | Blake3Ref,
  dependencies | Array Blake3Ref,
} in
let has_profile = fun profiles operation =>
  std.array.any (fun profile => profile.profile_ref == operation.profile_ref) profiles in
let profile_allows_operation = fun profiles operation =>
  std.array.all
    (fun profile =>
      profile.profile_ref != operation.profile_ref
      || profile.kind != "opaque"
      || (operation.kind != "diff" && operation.kind != "conflicts")
    )
    profiles in
let request_invariants = fun request =>
  std.array.length request.operations <= request.limits.max_operations
  && Prelude.non_empty_distinct_by (fun operation => operation.operation_id) request.operations
  && Prelude.non_empty_distinct_by (fun profile => profile.profile_ref) request.profiles
  && std.array.all
    (fun operation =>
      std.array.length operation.dependencies <= request.limits.max_dependencies_per_operation
      && has_profile request.profiles operation
      && profile_allows_operation request.profiles operation
    )
    request.operations in
let RequestContract = std.contract.Sequence [{
  schema | Prelude.contract (Prelude.is_exact_string "molten.world-workflow-request.v1"),
  request_ref | Blake3Ref,
  world_ref | Blake3Ref,
  branch_id | Prelude.contracts.NonEmptyString,
  expected_head | Blake3Ref,
  expected_generation | NonNegativeInteger,
  policy_ref | Blake3Ref,
  authority_observation_ref | Blake3Ref,
  limits | LimitsContract,
  profiles | std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array ProfileContract],
  observations | std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array ObservationContract],
  operations | std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array OperationContract],
}, Prelude.contract request_invariants] in
{
  Request = RequestContract,
  Limits = LimitsContract,
  Profile = ProfileContract,
  Observation = ObservationContract,
  Operation = OperationContract,
}
