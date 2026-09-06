# Typed Nickel contracts for lifecycle-store policy and checked configuration projections.

let NonEmptyString = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.length value > 0
) in

let NonEmptyArray = std.contract.from_predicate (fun value =>
  std.is_array value && std.array.length value > 0
) in

let PositiveNumber = std.contract.from_predicate (fun value =>
  std.is_number value && value > 0
) in

let NonNegativeNumber = std.contract.from_predicate (fun value =>
  std.is_number value && value >= 0
) in

let GuidanceSourceRoot = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "project" || value == "lifecycle")
) in

let ProjectFactAuthority = std.contract.from_predicate (fun value =>
  std.is_string value && value == "project_fact"
) in

let AdvisoryAuthority = std.contract.from_predicate (fun value =>
  std.is_string value && value == "advisory_guidance"
) in

let TrueOnly = std.contract.from_predicate (fun value =>
  std.is_bool value && value
) in

let FalseOnly = std.contract.from_predicate (fun value =>
  std.is_bool value && !value
) in

let Blake3Algorithm = std.contract.from_predicate (fun value =>
  std.is_string value && value == "BLAKE3"
) in

let Canonicalization = std.contract.from_predicate (fun value =>
  std.is_string value && value == "serde_json_compact_sorted_keys"
) in

let Blake3Hex = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.is_match "^[0-9a-f]{64}$" value
) in

let OptionalBlake3Hex = std.contract.from_predicate (fun value =>
  value == null || (std.is_string value && std.string.is_match "^[0-9a-f]{64}$" value)
) in

let StoreId = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.is_match "^[a-z0-9][a-z0-9._-]*$" value
) in

let OptionalStoreId = std.contract.from_predicate (fun value =>
  value == null || (std.is_string value && std.string.is_match "^[a-z0-9][a-z0-9._-]*$" value)
) in

let RemoteHint = std.contract.from_predicate (fun value =>
  value == null || (
    std.is_string value
    && std.string.is_match "^[A-Za-z0-9][A-Za-z0-9+._:/@~-]*$" value
  )
) in

let SafePolicyPath = std.contract.from_predicate (fun value =>
  std.is_string value
  && std.string.length value > 0
  && std.string.is_match "^[.A-Za-z0-9][A-Za-z0-9._/-]*$" value
) in

let EligibleRootCommands = std.contract.from_predicate (fun value =>
  value == [
    "status",
    "spec list",
    "spec show",
    "validate",
    "context",
    "workflow status",
    "workflow instructions",
    "doctor",
  ]
) in

let SafeExecutable = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.is_match "^[A-Za-z0-9][A-Za-z0-9._+-]*$" value
) in

let SafeArgument = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.is_match "^[-A-Za-z0-9][A-Za-z0-9._/:=+-]*$" value
) in

let IssueCode = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.is_match "^lifecycle_store\\.[a-z0-9_.-]+$" value
) in

let RootResolutionSource = std.contract.from_predicate (fun value =>
  std.is_string value && (
    value == "explicit_root"
    || value == "explicit_lifecycle_store"
    || value == "native_root"
    || value == "project_pointer"
    || value == "classic_fallback"
  )
) in

let MemberArgumentMode = std.contract.from_predicate (fun value =>
  std.is_string value && value == "positional_paths"
) in

let NonEmptyStrings = NonEmptyArray in

let StoreReference = {
  store_id | StoreId,
  remote_hint | RemoteHint,
} in

let OptionalStoreReference = std.contract.from_predicate (fun value =>
  value == null || std.is_record value
) in

let StoreIdentitySource = {
  schema | NonEmptyString,
  store_id | StoreId,
  remote_hint | RemoteHint,
  non_claims | NonEmptyStrings | Array NonEmptyString,
} in

let StoreIdentityProjection = {
  schema | NonEmptyString,
  store_id | StoreId,
  remote_hint | RemoteHint,
  source_blake3 | Blake3Hex,
  non_claims | NonEmptyStrings | Array NonEmptyString,
} in

let ContextReference = {
  id | StoreId,
  source_root | GuidanceSourceRoot,
  path | SafePolicyPath,
  required | Bool,
  priority | NonNegativeNumber,
  authority | ProjectFactAuthority,
  profiles | Array StoreId | default = [],
  artifacts | Array StoreId | default = [],
  operations | Array StoreId | default = [],
} in

let ArtifactGuidance = {
  id | StoreId,
  text | NonEmptyString,
  priority | NonNegativeNumber,
  authority | AdvisoryAuthority,
  profiles | Array StoreId | default = [],
  artifacts | Array StoreId | default = [],
} in

let OperationGuidance = {
  id | StoreId,
  text | NonEmptyString,
  priority | NonNegativeNumber,
  authority | AdvisoryAuthority,
  operations | Array StoreId | default = [],
} in

let ProjectDeclarationSource = {
  schema | NonEmptyString,
  lifecycle_store | OptionalStoreReference,
  references | Array StoreReference,
  context_refs | Array ContextReference | default = [],
  artifact_guidance | Array ArtifactGuidance | default = [],
  operation_guidance | Array OperationGuidance | default = [],
  non_claims | NonEmptyStrings | Array NonEmptyString,
} in

let ProjectDeclarationProjection = {
  schema | NonEmptyString,
  lifecycle_store | OptionalStoreReference,
  references | Array StoreReference,
  context_refs | Array ContextReference | default = [],
  artifact_guidance | Array ArtifactGuidance | default = [],
  operation_guidance | Array OperationGuidance | default = [],
  source_blake3 | Blake3Hex,
  non_claims | NonEmptyStrings | Array NonEmptyString,
} in

let LocatorEntry = {
  store_id | StoreId,
  checkout_path | NonEmptyString,
  store_identity | Blake3Hex,
} in

let LocatorState = {
  schema | NonEmptyString,
  entries | Array LocatorEntry,
  state_hash | Blake3Hex,
} in

let WorksetMember = {
  label | NonEmptyString,
  path | NonEmptyString,
} in

let WorksetEntry = {
  id | StoreId,
  members | NonEmptyArray | Array WorksetMember,
  primary_member | NonEmptyString,
  opener_profile | NonEmptyString,
} in

let WorksetState = {
  schema | NonEmptyString,
  worksets | Array WorksetEntry,
  state_hash | Blake3Hex,
} in

let RootResolutionReceipt = {
  source | RootResolutionSource,
  root | NonEmptyString,
  store_id | OptionalStoreId,
  store_identity | OptionalBlake3Hex,
  project_identity | OptionalBlake3Hex,
  diagnostics | Array NonEmptyString,
} in

let ReceiptIssue = {
  code | IssueCode,
  field_path | NonEmptyString,
  message | NonEmptyString,
} in

let LifecycleStoreReceipt = {
  schema | NonEmptyString,
  command | NonEmptyString,
  valid | Bool,
  verdict | NonEmptyString,
  evidence_class | NonEmptyString,
  resolution | RootResolutionReceipt,
  issues | Array ReceiptIssue,
  policy_hash | Blake3Hex,
  receipt_hash | Blake3Hex,
  portable_hash | Blake3Hex,
  non_claims | NonEmptyStrings | Array NonEmptyString,
} in

let NamedBounds = {
  max_store_id_bytes | PositiveNumber,
  max_remote_hint_bytes | PositiveNumber,
  max_references | PositiveNumber,
  max_locator_entries | PositiveNumber,
  max_workset_id_bytes | PositiveNumber,
  max_worksets | PositiveNumber,
  max_workset_members | PositiveNumber,
  max_path_bytes | PositiveNumber,
  max_context_specs_per_store | PositiveNumber,
  max_spec_title_bytes | PositiveNumber,
  max_purpose_summary_bytes | PositiveNumber,
  max_issue_count | PositiveNumber,
  max_non_claims | PositiveNumber,
} in

let ProjectionPolicy = {
  source_schema | NonEmptyString,
  projection_schema | NonEmptyString,
  source_path | SafePolicyPath,
  projection_path | SafePolicyPath,
  source_identity_field | NonEmptyString,
  runtime_nickel_evaluation | FalseOnly,
  stale_projection_rejected | TrueOnly,
  required_fields | NonEmptyStrings | Array NonEmptyString,
  required_non_claims | NonEmptyStrings | Array NonEmptyString,
} in

let RootResolutionPolicy = {
  root_flag | NonEmptyString,
  lifecycle_store_flag | NonEmptyString,
  selectors_mutually_exclusive | TrueOnly,
  eligible_commands | EligibleRootCommands,
  precedence | NonEmptyArray | Array RootResolutionSource,
  native_root_wins_over_project_pointer | TrueOnly,
  current_directory_only_without_vcs_boundary | TrueOnly,
  consult_global_registry | FalseOnly,
  machine_default_store | FalseOnly,
} in

let LocalStateSchema = {
  schema | NonEmptyString,
  relative_path | SafePolicyPath,
  state_hash_field | NonEmptyString,
  expected_prior_hash_field | NonEmptyString,
  hash_algorithm | Blake3Algorithm,
  preview_required | TrueOnly,
  lock_kind | NonEmptyString,
  replacement_kind | NonEmptyString,
  symlink_paths_allowed | FalseOnly,
} in

let OpenerProfile = {
  id | StoreId,
  executable | SafeExecutable,
  fixed_args | Array SafeArgument,
  member_argument_mode | MemberArgumentMode,
  primary_first | Bool,
} in

let WorksetPolicy = {
  state | LocalStateSchema,
  required_fields | NonEmptyStrings | Array NonEmptyString,
  minimum_members | PositiveNumber,
  opener_profile_field | NonEmptyString,
  removal_preserves_members | TrueOnly,
} in

let ReceiptPolicy = {
  schema | NonEmptyString,
  evidence_class | NonEmptyString,
  full_hash_field | NonEmptyString,
  portable_hash_field | NonEmptyString,
  required_fields | NonEmptyStrings | Array NonEmptyString,
  resolution_fields | NonEmptyStrings | Array NonEmptyString,
  local_display_fields | NonEmptyStrings | Array NonEmptyString,
  portable_fields | NonEmptyStrings | Array NonEmptyString,
  portable_excluded_fields | NonEmptyStrings | Array NonEmptyString,
  local_only_by_default | TrueOnly,
  release_readiness_consumes_by_default | FalseOnly,
  required_non_claims | NonEmptyStrings | Array NonEmptyString,
} in

let Policy = {
  schema | NonEmptyString,
  hash_algorithm | Blake3Algorithm,
  canonicalization | Canonicalization,
  bounds | NamedBounds,
  store_identity | ProjectionPolicy,
  project_config | ProjectionPolicy,
  root_resolution | RootResolutionPolicy,
  locator | LocalStateSchema,
  worksets | WorksetPolicy,
  opener_profiles | NonEmptyArray | Array OpenerProfile,
  receipts | ReceiptPolicy,
  issue_codes | NonEmptyArray | Array IssueCode,
  required_non_claims | NonEmptyStrings | Array NonEmptyString,
} in

let StoreIdContract = StoreId in
let RemoteHintContract = RemoteHint in
let StoreReferenceContract = StoreReference in
let StoreIdentitySourceContract = StoreIdentitySource in
let StoreIdentityProjectionContract = StoreIdentityProjection in
let ProjectDeclarationSourceContract = ProjectDeclarationSource in
let ProjectDeclarationProjectionContract = ProjectDeclarationProjection in
let LocatorEntryContract = LocatorEntry in
let LocatorStateContract = LocatorState in
let WorksetMemberContract = WorksetMember in
let WorksetEntryContract = WorksetEntry in
let WorksetStateContract = WorksetState in
let RootResolutionReceiptContract = RootResolutionReceipt in
let ReceiptIssueContract = ReceiptIssue in
let LifecycleStoreReceiptContract = LifecycleStoreReceipt in
let NamedBoundsContract = NamedBounds in
let ProjectionPolicyContract = ProjectionPolicy in
let RootResolutionPolicyContract = RootResolutionPolicy in
let LocalStateSchemaContract = LocalStateSchema in
let OpenerProfileContract = OpenerProfile in
let WorksetPolicyContract = WorksetPolicy in
let ReceiptPolicyContract = ReceiptPolicy in
let PolicyContract = Policy in

{
  StoreId = StoreIdContract,
  RemoteHint = RemoteHintContract,
  StoreReference = StoreReferenceContract,
  StoreIdentitySource = StoreIdentitySourceContract,
  StoreIdentityProjection = StoreIdentityProjectionContract,
  ContextReference = ContextReference,
  ArtifactGuidance = ArtifactGuidance,
  OperationGuidance = OperationGuidance,
  ProjectDeclarationSource = ProjectDeclarationSourceContract,
  ProjectDeclarationProjection = ProjectDeclarationProjectionContract,
  LocatorEntry = LocatorEntryContract,
  LocatorState = LocatorStateContract,
  WorksetMember = WorksetMemberContract,
  WorksetEntry = WorksetEntryContract,
  WorksetState = WorksetStateContract,
  RootResolutionReceipt = RootResolutionReceiptContract,
  ReceiptIssue = ReceiptIssueContract,
  LifecycleStoreReceipt = LifecycleStoreReceiptContract,
  NamedBounds = NamedBoundsContract,
  ProjectionPolicy = ProjectionPolicyContract,
  RootResolutionPolicy = RootResolutionPolicyContract,
  LocalStateSchema = LocalStateSchemaContract,
  OpenerProfile = OpenerProfileContract,
  WorksetPolicy = WorksetPolicyContract,
  ReceiptPolicy = ReceiptPolicyContract,
  Policy = PolicyContract,
}
