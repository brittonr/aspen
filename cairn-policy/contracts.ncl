# Nickel contracts for cairn policy profiles.

let Prelude = import "../docs/nickel-contract-prelude.ncl" in
let PolicyVersion = 1 in
let ReceiptHashAlgorithm = "BLAKE3" in
let ReceiptHashCanonicalization = "serde_json_compact_sorted_keys" in
let Blake3HexPattern = "^[0-9a-fA-F]{64}$" in

let NonEmptyString = Prelude.contracts.NonEmptyString in
let NonEmptyArray = Prelude.contracts.NonEmptyArray in
let NonEmptyUniqueStringArray = Prelude.contracts.NonEmptyUniqueStringArray in
let UniqueStringArray = Prelude.contracts.UniqueStringArray in
let SupportedPolicyVersion = Prelude.contract (Prelude.is_exact_integer PolicyVersion) in
let Blake3Algorithm = Prelude.contract (Prelude.is_exact_string ReceiptHashAlgorithm) in
let Canonicalization = Prelude.contract (Prelude.is_exact_string ReceiptHashCanonicalization) in
let Blake3HexContract = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match Blake3HexPattern value
) in

let all_artifacts = fun policy =>
  std.array.flatten (std.array.map (fun schema => schema.artifacts) policy.schemas) in
let artifact_ids = fun policy =>
  std.array.map (fun artifact => artifact.id) (all_artifacts policy) in
let artifact_requires = fun policy =>
  std.array.flatten (std.array.map (fun artifact => artifact.requires) (all_artifacts policy)) in
let marker_ids = fun policy =>
  std.array.map (fun marker => marker.id) policy.task_marker_policy.markers in
let marker_tokens = fun policy =>
  std.array.map (fun marker => marker.token) policy.task_marker_policy.markers in
let replay_case_ids = fun policy =>
  std.array.map (fun replay_case => replay_case.id) policy.determinism_replay_policy.cases in
let replay_group_ids = fun policy =>
  std.array.map (fun replay_group => replay_group.id) policy.determinism_replay_policy.groups in
let replay_group_cases = fun policy =>
  std.array.flatten (std.array.map (fun replay_group => replay_group.cases) policy.determinism_replay_policy.groups) in
let surface_ids = fun policy =>
  std.array.map (fun surface => surface.id) policy.determinism_surface_inventory in
let surface_replay_cases = fun policy =>
  std.array.flatten (std.array.map (fun surface => surface.required_replay_cases) policy.determinism_surface_inventory) in
let surface_replay_groups = fun policy =>
  std.array.flatten (std.array.map (fun surface => surface.required_replay_groups) policy.determinism_surface_inventory) in
let receipt_schema_commands = fun policy =>
  std.array.map (fun schema => schema.command) policy.receipt_schemas in
let receipt_contract_commands = fun policy =>
  std.array.map (fun contract => contract.command) policy.receipt_contracts in
let all_known = fun known_ids refs =>
  std.array.all (fun reference => std.array.elem reference known_ids) refs in
let policy_references_resolve = fun policy =>
  all_known (artifact_ids policy) (artifact_requires policy)
  && all_known (replay_case_ids policy) (replay_group_cases policy)
  && all_known (replay_case_ids policy) (surface_replay_cases policy)
  && all_known (replay_group_ids policy) (surface_replay_groups policy)
  && all_known (receipt_schema_commands policy) (receipt_contract_commands policy) in
let policy_ids_are_unique = fun policy =>
  Prelude.non_empty_distinct_by (fun schema => schema.id) policy.schemas
  && Prelude.distinct_strings (artifact_ids policy)
  && Prelude.non_empty_distinct_by (fun marker => marker.id) policy.task_marker_policy.markers
  && Prelude.non_empty_distinct_by (fun marker => marker.token) policy.task_marker_policy.markers
  && Prelude.non_empty_distinct_by (fun replay_case => replay_case.id) policy.determinism_replay_policy.cases
  && Prelude.non_empty_distinct_by (fun replay_group => replay_group.id) policy.determinism_replay_policy.groups
  && Prelude.non_empty_distinct_by (fun surface => surface.id) policy.determinism_surface_inventory
  && Prelude.non_empty_distinct_by (fun schema => schema.command) policy.receipt_schemas
  && Prelude.distinct_strings (receipt_contract_commands policy) in
let policy_invariants_hold = fun policy =>
  std.is_record policy
  && policy_references_resolve policy
  && policy_ids_are_unique policy in
let PolicyInvariants = Prelude.contract policy_invariants_hold in
let RequiredExemptionFields = ["reason", "owner", "until"] in
let RequiredExemptionDateFields = ["until"] in
let exemption_marker_policy_valid = fun policy =>
  std.is_record policy
  && Prelude.has_all policy.required_fields RequiredExemptionFields
  && Prelude.has_all policy.date_fields RequiredExemptionDateFields in
let ExemptionMarkerPolicyInvariants = Prelude.contract exemption_marker_policy_valid in

let Stage = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "proposal" || value == "design" || value == "tasks" || value == "implementation" || value == "archive")
) in

let Strictness = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "strict" || value == "warn" || value == "off")
) in

let MarkerToken = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.is_match "^\\[(parallel|serial|depends:[^\\]]+)\\]$" value
) in

let MarkerKind = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "literal" || value == "depends-prefix")
) in

let Severity = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "low" || value == "medium" || value == "high" || value == "critical")
) in

let FindingClass = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "omission" || value == "systematic" || value == "incoherent" || value == "preference" || value == "env" || value == "pattern")
) in

let FindingScope = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "proposal" || value == "spec" || value == "design" || value == "tasks" || value == "review" || value == "tooling")
) in

let FindingRoute = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "auto-fix" || value == "prompt" || value == "deterministic-check" || value == "spec-rule" || value == "human" || value == "guidance")
) in

let Promoted = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "yes" || value == "no")
) in

let SteelProfileMode = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "disabled" || value == "comparison" || value == "opt_in" || value == "selected_default" || value == "broad_default")
) in
let SafeRelativePath = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.length value > 0 && std.string.is_match "^[A-Za-z0-9][A-Za-z0-9._/-]*$" value
) in
let TraceabilityRequirementSourceKind = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "accepted_specs" || value == "requirements_doc")
) in
let TraceabilityEvidenceSourceKind = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "rust_source" || value == "nickel_source" || value == "tool_source")
) in
let TraceabilityMarkerVerb = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "impl" || value == "verify" || value == "depends" || value == "related")
) in
let TraceabilityExtension = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.length value > 0 && std.string.is_match "^[A-Za-z0-9_-]+$" value
) in
let StackProvenanceGateMode = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "disabled" || value == "optional" || value == "required")
) in
let RuntimeCaptureOrigin = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "boundary_captured" || value == "ingested_telemetry" || value == "wrapper_asserted" || value == "unknown")
) in
let PositiveNumber = Prelude.contracts.PositiveInteger in

{
  Artifact = {
    id | NonEmptyString,
    generates | NonEmptyString,
    requires | Array NonEmptyString | default = [],
  },

  ArtifactSchema = {
    id | NonEmptyString,
    description | NonEmptyString,
    artifacts | Array Artifact,
  },

  Marker = {
    id | NonEmptyString,
    token | MarkerToken,
    kind | MarkerKind,
  },

  TaskMarkerPolicy = {
    enabled | Bool,
    stages | NonEmptyUniqueStringArray | Array Stage,
    strictness | Strictness,
    ignore_archives | Bool,
    markers | NonEmptyArray | Array Marker,
  },

  FindingPolicy = {
    severity | Severity,
    class | FindingClass,
    scope | FindingScope,
    route | FindingRoute,
    promoted | Promoted,
  },

  GatePolicy = {
    stages | NonEmptyArray | Array Stage,
    ignore_archives | Bool,
    finding | FindingPolicy,
  },

  ReportContract = {
    verdicts | NonEmptyUniqueStringArray | Array NonEmptyString,
    sections | NonEmptyUniqueStringArray | Array NonEmptyString,
    tagged_findings_required_fields | NonEmptyUniqueStringArray | Array NonEmptyString,
  },

  RegistryLabelPolicy = {
    channels | NonEmptyUniqueStringArray | Array NonEmptyString,
    platforms | NonEmptyUniqueStringArray | Array NonEmptyString,
    compatibility_classes | NonEmptyUniqueStringArray | Array NonEmptyString,
    release_format | NonEmptyString,
  },

  ChangeMetadataPolicy = {
    allowed_groups | NonEmptyUniqueStringArray | Array NonEmptyString,
    group_prefixes | UniqueStringArray | Array NonEmptyString | default = [],
    statuses | NonEmptyUniqueStringArray | Array NonEmptyString,
  },

  SteelOrchestrationProfile = {
    id | NonEmptyString,
    mode | SteelProfileMode,
    allowed_host_functions | UniqueStringArray | Array NonEmptyString | default = [],
    deterministic_budgets | NonEmptyUniqueStringArray | Array NonEmptyString,
    redaction_classes | NonEmptyUniqueStringArray | Array NonEmptyString,
    fallback_modes | NonEmptyUniqueStringArray | Array NonEmptyString,
    receipt_schema_version | NonEmptyString,
  },

  SteelOrchestrationPolicy = {
    enabled | Bool,
    profiles | Array SteelOrchestrationProfile | default = [],
  },

  TraceabilityRequirementSource = {
    root | SafeRelativePath,
    source_kind | TraceabilityRequirementSourceKind,
    extensions | NonEmptyUniqueStringArray | Array TraceabilityExtension,
  },

  TraceabilityEvidenceSource = {
    root | SafeRelativePath,
    source_kind | TraceabilityEvidenceSourceKind,
    extensions | NonEmptyUniqueStringArray | Array TraceabilityExtension,
  },

  TraceabilityProfile = {
    id | NonEmptyString,
    requirement_sources | NonEmptyArray | Array TraceabilityRequirementSource,
    evidence_sources | NonEmptyArray | Array TraceabilityEvidenceSource,
    marker_verbs | NonEmptyUniqueStringArray | Array TraceabilityMarkerVerb,
  },

  TraceabilityPolicy = {
    default_profile | NonEmptyString,
    profiles | NonEmptyArray | Array TraceabilityProfile,
  },

  StackProvenancePolicy = {
    mode | StackProvenanceGateMode,
    allowed_claim_scopes | NonEmptyUniqueStringArray | Array NonEmptyString,
    accepted_valence_policy_hashes | UniqueStringArray | Array Blake3HexContract | default = [],
    required_boundary | NonEmptyString,
  },

  FunctionAddressPolicy = {
    mode | StackProvenanceGateMode,
    allowed_claim_scopes | NonEmptyUniqueStringArray | Array NonEmptyString,
    accepted_valence_policy_hashes | UniqueStringArray | Array Blake3HexContract | default = [],
    required_roles | NonEmptyUniqueStringArray | Array NonEmptyString,
    required_schemas | NonEmptyUniqueStringArray | Array NonEmptyString,
    required_boundary | NonEmptyString,
  },

  RuntimeEvidencePolicy = {
    allowed_capture_origins | NonEmptyUniqueStringArray | Array RuntimeCaptureOrigin,
    allowed_hash_algorithms | NonEmptyUniqueStringArray | Array Blake3Algorithm,
    max_summary_bytes | PositiveNumber,
    forbidden_summary_fragments | UniqueStringArray | Array NonEmptyString | default = [],
    aggregation_profiles | UniqueStringArray | Array NonEmptyString | default = [],
  },

  ProbeCommandProfile = {
    id | NonEmptyString,
    commands | NonEmptyArray | Array NonEmptyString,
    capability_classes | UniqueStringArray | Array NonEmptyString | default = [],
  },

  ProbePolicy = {
    enabled | Bool,
    inline_gate_execution_enabled | Bool,
    receipt_schema_version | NonEmptyString,
    coverage_classes | NonEmptyUniqueStringArray | Array NonEmptyString,
    command_profiles | NonEmptyArray | Array ProbeCommandProfile,
    redaction_classes | NonEmptyUniqueStringArray | Array NonEmptyString,
  },

  ReceiptSchema = {
    command | NonEmptyString,
    top_level_keys | NonEmptyUniqueStringArray | Array NonEmptyString,
    action_keys | UniqueStringArray | Array NonEmptyString | default = [],
    issue_keys | UniqueStringArray | Array NonEmptyString | default = [],
    case_keys | UniqueStringArray | Array NonEmptyString | default = [],
    mismatch_keys | UniqueStringArray | Array NonEmptyString | default = [],
    artifact_keys | UniqueStringArray | Array NonEmptyString | default = [],
    replay_evidence_keys | UniqueStringArray | Array NonEmptyString | default = [],
    hash_fields | UniqueStringArray | Array NonEmptyString | default = [],
  },

  ReceiptContract = {
    family | NonEmptyString,
    schema | NonEmptyString,
    command | NonEmptyString,
    ownership | NonEmptyString,
    compatibility_class | NonEmptyString,
    encoding | NonEmptyString,
    hash_field | NonEmptyString,
    hash_material | NonEmptyString,
    non_claims | NonEmptyUniqueStringArray | Array NonEmptyString,
  },

  ReplayCase = {
    id | NonEmptyString,
    command_kind | NonEmptyString,
    args | NonEmptyUniqueStringArray | Array NonEmptyString,
    fixture_root | NonEmptyString,
    expected_mutability | NonEmptyString,
    comparison_modes | NonEmptyUniqueStringArray | Array NonEmptyString,
    display_only_paths | UniqueStringArray | Array NonEmptyString | default = [],
    hash_fields | UniqueStringArray | Array NonEmptyString | default = [],
    normalization_rules | NonEmptyUniqueStringArray | Array NonEmptyString,
  },

  ReplayGroup = {
    id | NonEmptyString,
    description | NonEmptyString,
    cases | NonEmptyUniqueStringArray | Array NonEmptyString,
  },

  EntropyInventoryEntry = {
    id | NonEmptyString,
    class | NonEmptyString,
    source | NonEmptyString,
    handling | NonEmptyString,
    receipt_field | NonEmptyString,
    description | NonEmptyString,
  },

  EnvironmentAttestationFact = {
    id | NonEmptyString,
    class | NonEmptyString,
    source | NonEmptyString,
    handling | NonEmptyString,
    receipt_field | NonEmptyString,
    description | NonEmptyString,
  },

  DeterminismReplayPolicy = {
    cases | NonEmptyArray | Array ReplayCase,
    groups | NonEmptyArray | Array ReplayGroup,
    entropy_inventory | NonEmptyArray | Array EntropyInventoryEntry,
    environment_attestation | NonEmptyArray | Array EnvironmentAttestationFact,
  },

  DeterminismSurface = {
    id | NonEmptyString,
    command | NonEmptyString,
    surface_kind | NonEmptyString,
    receipt_command | NonEmptyString,
    required_replay_cases | NonEmptyUniqueStringArray | Array NonEmptyString,
    required_replay_groups | NonEmptyUniqueStringArray | Array NonEmptyString,
    determinism_claim | NonEmptyString,
  },

  ExemptionMarkerPolicy = std.contract.Sequence [{
    required_fields | NonEmptyUniqueStringArray | Array NonEmptyString,
    date_fields | NonEmptyUniqueStringArray | Array NonEmptyString,
    optional_fields | UniqueStringArray | Array NonEmptyString | default = [],
  }, ExemptionMarkerPolicyInvariants],

  SyncArchivePolicy = {
    require_all_tasks_done | Bool,
    archive_date_prefix | Bool,
    default_archive_dir | NonEmptyString,
  },

  MutationGate = {
    require_passing_gate | Bool,
  },

  MigrationPolicy = {
    require_manifest | Bool,
  },

  MutationPolicy = {
    sync | MutationGate,
    archive | MutationGate,
    migrate | MigrationPolicy,
  },

  ReceiptHashPolicy = {
    required | Bool,
    algorithm | Blake3Algorithm,
    canonicalization | Canonicalization,
  },

  Exemption = {
    scope | NonEmptyString,
    owner | NonEmptyString,
    reason | NonEmptyString,
    expires | NonEmptyString,
  },

  PreservesReceiptSourceMap = { _ : NonEmptyString },

  StructuredLifecycle = {
    preserves_receipt_source_map | PreservesReceiptSourceMap | default = {},
  },

  PolicySchemaCompatibility = {
    schema_version | NonEmptyString,
    producer | NonEmptyString,
    field_set_digest | Blake3HexContract,
    refresh_command | NonEmptyString,
  },

  Policy = std.contract.Sequence [{
    name | NonEmptyString,
    version | SupportedPolicyVersion,
    policy_schema_compatibility | PolicySchemaCompatibility,
    schemas | NonEmptyArray | Array ArtifactSchema,
    lifecycle_stages | NonEmptyArray | Array Stage,
    task_marker_policy | TaskMarkerPolicy,
    gate_policy | GatePolicy,
    finding_defaults | FindingPolicy,
    report_contract | ReportContract,
    registry_label_policy | RegistryLabelPolicy,
    change_metadata_policy | ChangeMetadataPolicy,
    steel_orchestration_policy | SteelOrchestrationPolicy,
    traceability_policy | TraceabilityPolicy,
    stack_provenance_gate | StackProvenancePolicy,
    function_address_gate | FunctionAddressPolicy,
    runtime_evidence_policy | RuntimeEvidencePolicy,
    structured_lifecycle | StructuredLifecycle | default = {},
    probe_policy | ProbePolicy,
    receipt_schemas | NonEmptyArray | Array ReceiptSchema,
    receipt_contracts | Array ReceiptContract | default = [],
    determinism_replay_policy | DeterminismReplayPolicy,
    determinism_surface_inventory | NonEmptyArray | Array DeterminismSurface,
    sync_archive_policy | SyncArchivePolicy,
    mutation_policy | MutationPolicy,
    receipt_hash_policy | ReceiptHashPolicy,
    exemption_marker_policy | ExemptionMarkerPolicy,
    exemptions | Array Exemption,
  }, PolicyInvariants],

  Blake3Hex = Blake3HexContract,
}
