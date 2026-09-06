# Nickel contracts for cairn policy profiles.

let LifecycleStores = import "lifecycle-store-contracts.ncl" in

let NonEmptyString = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.length value > 0
) in

let NonEmptyArray = std.contract.from_predicate (fun value =>
  std.is_array value && std.array.length value > 0
) in

let PositiveNumber = std.contract.from_predicate (fun value =>
  std.is_number value && value > 0
) in

let SupportedPolicyVersion = std.contract.from_predicate (fun value =>
  std.is_number value && value == 1
) in

let Blake3Algorithm = std.contract.from_predicate (fun value =>
  std.is_string value && value == "BLAKE3"
) in

let Canonicalization = std.contract.from_predicate (fun value =>
  std.is_string value && value == "serde_json_compact_sorted_keys"
) in

let Blake3HexContract = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.is_match "^[0-9a-fA-F]{64}$" value
) in

let Stage = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "proposal" || value == "design" || value == "tasks" || value == "implementation" || value == "archive")
) in

let Strictness = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "strict" || value == "warn" || value == "off")
) in

let MarkerToken = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.is_match "^\\[(parallel|serial|depends:[^\\]]+|task:[^\\]]+|after:[^\\]]+)\\]$" value
) in

let MarkerKind = std.contract.from_predicate (fun value =>
  std.is_string value
  && (value == "literal"
    || value == "depends-prefix"
    || value == "task-id-prefix"
    || value == "task-predecessor-prefix")
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

let MaxMachineProcessStatus = 125 in

let MachineExitClass = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "success" || value == "invalid_arguments" || value == "not_found" || value == "policy_error" || value == "blocked" || value == "internal_error")
) in

let MachineDiagnosticSeverity = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "error" || value == "warning" || value == "info")
) in

let MachineProcessStatus = std.contract.from_predicate (fun value =>
  std.is_number value && value >= 0 && value <= MaxMachineProcessStatus
) in

let MachineDiagnosticCode = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.length value > 0 && std.string.is_match "^[a-z0-9][a-z0-9._-]*$" value
) in

let MachineDiagnosticTarget = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.length value > 0 && std.string.is_match "^[A-Za-z0-9][A-Za-z0-9._/-]*$" value
) in

let SteelProfileMode = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "disabled" || value == "comparison" || value == "opt_in" || value == "selected_default" || value == "broad_default")
) in

# r[impl cairn.lifecycle_directory.hidden_default]
let SafeRelativePath = std.contract.from_predicate (fun value =>
  std.is_string value
  && std.string.length value > 0
  && (
    std.string.is_match "^[A-Za-z0-9][A-Za-z0-9._/-]*$" value
    || std.string.is_match "^\\.cairn(/[A-Za-z0-9][A-Za-z0-9._/-]*)?$" value
  )
) in

let WorkflowState = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "done" || value == "ready" || value == "blocked" || value == "skipped")
) in

let WorkflowPathKind = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "file" || value == "spec_directory")
) in

let WorkflowStructuralAdmission = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "proposal" || value == "delta_specs" || value == "design" || value == "tasks" || value == "markdown" || value == "acceptance" || value == "risk" || value == "verification" || value == "semantic_review")
) in

let WorkflowSpecEffect = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "delta" || value == "none")
) in

let WorkflowTaskLinkClass = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "requirement" || value == "acceptance")
) in

let WorkflowReviewKind = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "plan" || value == "design" || value == "code" || value == "diff")
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

let TraceabilityAssuranceLevel = std.contract.from_predicate (fun value =>
  std.is_string value && value == "declared"
) in

let TraceabilityAnchorKind = std.contract.from_predicate (fun value =>
  std.is_string value && value == "exact_marker_bytes"
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

let PositiveNumber = std.contract.from_predicate (fun value =>
  std.is_number value && value > 0
) in

let DependencyTargetClass = std.contract.from_predicate (fun value =>
  std.is_string value && value == "lifecycle-id"
) in

let NominalIdentityCategory = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "domain-id" || value == "domain-ref" || value == "blake3-digest" || value == "imported-digest")
) in

let NominalIdentityAlgorithm = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "none" || value == "BLAKE3" || value == "SHA-256" || value == "Git-OID-SHA-1")
) in

let NominalIdentityRawRepresentation = std.contract.from_predicate (fun value =>
  std.is_string value && value == "string"
) in

let NominalIdentityWrapperKind = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "refined-newtype" || value == "marker-generic")
) in

let NominalIdentityConstructorPolicy = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "private-fallible" || value == "marker-only")
) in

let NominalIdentityOctetMode = std.contract.from_predicate (fun value =>
  std.is_string value && value == "deny"
) in

let GeneratedWorkflowPermission = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "read" || value == "mutation")
) in

let GeneratedWorkflowMutationClass = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "read_only" || value == "preview_first")
) in

let GeneratedWorkflowAdapterFormat = std.contract.from_predicate (fun value =>
  std.is_string value && (
    value == "agent-skills" || value == "claude" || value == "codex"
  )
) in

let DeveloperLoopEffectPosture = std.contract.from_predicate (fun value =>
  std.is_string value && (
    value == "read_only" ||
    value == "read_write_file" ||
    value == "network_read" ||
    value == "network_write"
  )
) in

let DeveloperLoopCacheMode = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "disabled" || value == "enabled")
) in

let DeveloperLoopStatusClass = std.contract.from_predicate (fun value =>
  std.is_string value && (value == "success" || value == "failure")
) in

{
  WorkflowBounds = {
    max_artifacts | PositiveNumber,
    max_dependencies | PositiveNumber,
    max_files_per_artifact | PositiveNumber,
    max_path_bytes | PositiveNumber,
    max_artifact_bytes | PositiveNumber,
    max_template_bytes | PositiveNumber,
    max_constraints | PositiveNumber,
    max_non_claims | PositiveNumber,
    max_issues | PositiveNumber,
    max_directory_depth | PositiveNumber,
    max_directory_entries | PositiveNumber,
  },

  Artifact = {
    id | NonEmptyString,
    generates | NonEmptyString,
    output_path | SafeRelativePath,
    path_kind | WorkflowPathKind,
    structural_admission | WorkflowStructuralAdmission,
    requires | Array NonEmptyString | default = [],
    stage | Stage,
    instruction_template | NonEmptyString,
    constraints | Array NonEmptyString | default = [],
    apply_prerequisites | Array NonEmptyString | default = [],
    skipped | Bool | default = false,
    skip_satisfies_dependencies | Bool | default = false,
  },

  WorkflowReviewObligation = {
    id | NonEmptyString,
    artifact_id | NonEmptyString,
    review_kind | WorkflowReviewKind,
    required_artifacts | NonEmptyArray | Array NonEmptyString,
  },

  ArtifactSchema = {
    schema_version | NonEmptyString,
    id | NonEmptyString,
    profile_version | PositiveNumber,
    description | NonEmptyString,
    spec_effect | WorkflowSpecEffect,
    task_link_class | WorkflowTaskLinkClass,
    states | NonEmptyArray | Array WorkflowState,
    bounds | WorkflowBounds,
    artifacts | NonEmptyArray | Array Artifact,
    review_obligations | Array WorkflowReviewObligation | default = [],
    receipt_fields | NonEmptyArray | Array NonEmptyString,
    required_non_claims | NonEmptyArray | Array NonEmptyString,
  },

  WorkflowEffectProfile = {
    effect | WorkflowSpecEffect,
    profile | NonEmptyString,
  },

  WorkflowProfilePolicy = {
    default_profile | NonEmptyString,
    allowed_profiles | NonEmptyArray | Array NonEmptyString,
    required_profiles_by_effect | Array WorkflowEffectProfile | default = [],
    forbidden_selection_sources | NonEmptyArray | Array NonEmptyString,
  },

  AgentGuidanceBounds = {
    max_context_refs | PositiveNumber,
    max_artifact_guidance | PositiveNumber,
    max_operation_guidance | PositiveNumber,
    max_selectors_per_entry | PositiveNumber,
    max_id_bytes | PositiveNumber,
    max_path_bytes | PositiveNumber,
    max_inline_text_bytes | PositiveNumber,
    max_source_text_bytes | PositiveNumber,
    max_priority | PositiveNumber,
    max_issues | PositiveNumber,
  },

  AgentGuidancePolicy = {
    schema | NonEmptyString,
    allowed_source_roots | NonEmptyArray | Array NonEmptyString,
    allowed_authority_classes | NonEmptyArray | Array NonEmptyString,
    known_profiles | NonEmptyArray | Array NonEmptyString,
    known_artifacts | NonEmptyArray | Array NonEmptyString,
    known_operations | NonEmptyArray | Array NonEmptyString,
    bounds | AgentGuidanceBounds,
    receipt_fields | NonEmptyArray | Array NonEmptyString,
    portable_excluded_fields | NonEmptyArray | Array NonEmptyString,
    required_non_claims | NonEmptyArray | Array NonEmptyString,
  },

  GeneratedWorkflowBounds = {
    max_descriptors | PositiveNumber,
    max_adapters | PositiveNumber,
    max_command_classes | PositiveNumber,
    max_text_bytes | PositiveNumber,
    max_list_items | PositiveNumber,
    max_files_per_adapter | PositiveNumber,
    max_path_bytes | PositiveNumber,
    max_file_bytes | PositiveNumber,
    max_issues | PositiveNumber,
  },

  GeneratedWorkflowCommandClass = {
    id | NonEmptyString,
    permission | GeneratedWorkflowPermission,
  },

  GeneratedWorkflowDescriptor = {
    id | NonEmptyString,
    version | PositiveNumber,
    goal | NonEmptyString,
    success_evidence | NonEmptyArray | Array NonEmptyString,
    required_evidence | NonEmptyArray | Array NonEmptyString,
    command_classes | NonEmptyArray | Array NonEmptyString,
    mutation_class | GeneratedWorkflowMutationClass,
    progress_rule | NonEmptyString,
    immediate_action | NonEmptyString,
    output_shape | NonEmptyString,
    constraints | NonEmptyArray | Array NonEmptyString,
    non_claims | NonEmptyArray | Array NonEmptyString,
  },

  GeneratedWorkflowAdapter = {
    id | NonEmptyString,
    version | PositiveNumber,
    format | GeneratedWorkflowAdapterFormat,
    file_prefix | NonEmptyString,
    file_suffix | String,
    directory_suffix | String,
    front_matter | Bool,
    max_files | PositiveNumber,
  },

  GeneratedAgentWorkflowPolicy = {
    schema | NonEmptyString,
    generator_version | NonEmptyString,
    command_classes | NonEmptyArray | Array GeneratedWorkflowCommandClass,
    descriptors | NonEmptyArray | Array GeneratedWorkflowDescriptor,
    adapters | NonEmptyArray | Array GeneratedWorkflowAdapter,
    bounds | GeneratedWorkflowBounds,
    receipt_fields | NonEmptyArray | Array NonEmptyString,
    required_non_claims | NonEmptyArray | Array NonEmptyString,
  },

  DeveloperLoopBounds = {
    max_profiles | PositiveNumber,
    max_budgets | PositiveNumber,
    max_checks | PositiveNumber,
    max_checks_per_profile | PositiveNumber,
    max_text_bytes | PositiveNumber,
    max_issues | PositiveNumber,
    max_cache_key_inputs | PositiveNumber,
    max_duration_seconds | PositiveNumber,
    max_receipt_bytes | PositiveNumber,
  },

  DeveloperLoopBudget = {
    id | NonEmptyString,
    max_seconds | PositiveNumber,
  },

  DeveloperLoopCheck = {
    id | NonEmptyString,
    command_identity | NonEmptyString,
    argv | NonEmptyArray | Array NonEmptyString,
    working_root | NonEmptyString,
    expected_status_class | DeveloperLoopStatusClass,
  },

  DeveloperLoopCachePolicy = {
    mode | DeveloperLoopCacheMode,
    key_inputs | Array NonEmptyString | default = [],
  },

  DeveloperLoopProfile = {
    id | NonEmptyString,
    purpose | NonEmptyString,
    repositories | NonEmptyArray | Array NonEmptyString,
    subject_scope | NonEmptyString,
    checks | NonEmptyArray | Array NonEmptyString,
    target | NonEmptyString,
    toolchain | NonEmptyString,
    budget_id | NonEmptyString,
    effect_posture | DeveloperLoopEffectPosture,
    cache_policy | DeveloperLoopCachePolicy,
    full_gate_refs | NonEmptyArray | Array NonEmptyString,
    required_enforcement | Array NonEmptyString | default = [],
    non_claims | NonEmptyArray | Array NonEmptyString,
  },

  DeveloperLoopPolicy = {
    schema | NonEmptyString,
    profiles | NonEmptyArray | Array DeveloperLoopProfile,
    budgets | NonEmptyArray | Array DeveloperLoopBudget,
    checks | NonEmptyArray | Array DeveloperLoopCheck,
    bounds | DeveloperLoopBounds,
    receipt_fields | NonEmptyArray | Array NonEmptyString,
    required_non_claims | NonEmptyArray | Array NonEmptyString,
  },

  DocumentCollaborationBounds = {
    max_document_id_bytes | PositiveNumber,
    max_artifacts | PositiveNumber,
    max_discussion_messages | PositiveNumber,
    max_discussion_text_bytes | PositiveNumber,
    max_links | PositiveNumber,
    max_revisions_per_artifact | PositiveNumber,
    max_path_bytes | PositiveNumber,
    max_number | PositiveNumber,
  },

  DocumentCollaborationProfile = {
    bounds | DocumentCollaborationBounds,
    required_non_claims | NonEmptyArray | Array NonEmptyString,
  },

  DocumentCollaborationPolicy = {
    schema | NonEmptyString,
    kinds | NonEmptyArray | Array NonEmptyString,
    profile | DocumentCollaborationProfile,
  },

  Marker = {
    id | NonEmptyString,
    token | MarkerToken,
    kind | MarkerKind,
  },

  TaskMarkerPolicy = {
    enabled | Bool,
    stages | NonEmptyArray | Array Stage,
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

  ArtifactSubstancePolicy = {
    proposal_minimum_substantive_lines | PositiveNumber,
    design_minimum_substantive_lines | PositiveNumber,
    minimum_requirement_blocks | PositiveNumber,
    minimum_substantive_tasks | PositiveNumber,
    placeholder_tokens | NonEmptyArray | Array NonEmptyString,
    normative_tokens | NonEmptyArray | Array NonEmptyString,
    dependency_marker_id | NonEmptyString,
    dependency_target_class | DependencyTargetClass,
  },

  GatePolicy = {
    stages | NonEmptyArray | Array Stage,
    ignore_archives | Bool,
    finding | FindingPolicy,
    substance | ArtifactSubstancePolicy,
  },

  ReportContract = {
    verdicts | NonEmptyArray | Array NonEmptyString,
    sections | NonEmptyArray | Array NonEmptyString,
    tagged_findings_required_fields | NonEmptyArray | Array NonEmptyString,
  },

  MachineExitClassPolicy = {
    id | MachineExitClass,
    process_status | MachineProcessStatus,
  },

  MachineDiagnosticPolicy = {
    code | MachineDiagnosticCode,
    exit_class | MachineExitClass,
    severity | MachineDiagnosticSeverity,
    message | NonEmptyString,
    target | MachineDiagnosticTarget,
    fix | String,
  },

  MachineContractBounds = {
    max_command_bytes | PositiveNumber,
    max_diagnostics | PositiveNumber,
    max_code_bytes | PositiveNumber,
    max_message_bytes | PositiveNumber,
    max_target_bytes | PositiveNumber,
    max_fix_bytes | PositiveNumber,
    max_stdout_bytes | PositiveNumber,
    max_stderr_bytes | PositiveNumber,
  },

  MachineContractPolicy = {
    schema_version | NonEmptyString,
    exit_classes | NonEmptyArray | Array MachineExitClassPolicy,
    diagnostic_fields | NonEmptyArray | Array NonEmptyString,
    diagnostic_order | NonEmptyArray | Array MachineDiagnosticCode,
    diagnostics | NonEmptyArray | Array MachineDiagnosticPolicy,
    bounds | MachineContractBounds,
    required_non_claims | NonEmptyArray | Array NonEmptyString,
  },

  RegistryLabelPolicy = {
    channels | NonEmptyArray | Array NonEmptyString,
    platforms | NonEmptyArray | Array NonEmptyString,
    compatibility_classes | NonEmptyArray | Array NonEmptyString,
    release_format | NonEmptyString,
  },

  ChangeMetadataPolicy = {
    allowed_groups | NonEmptyArray | Array NonEmptyString,
    group_prefixes | Array NonEmptyString | default = [],
    statuses | NonEmptyArray | Array NonEmptyString,
  },

  SteelOrchestrationProfile = {
    id | NonEmptyString,
    mode | SteelProfileMode,
    allowed_host_functions | Array NonEmptyString | default = [],
    deterministic_budgets | NonEmptyArray | Array NonEmptyString,
    redaction_classes | NonEmptyArray | Array NonEmptyString,
    fallback_modes | NonEmptyArray | Array NonEmptyString,
    receipt_schema_version | NonEmptyString,
  },

  SteelOrchestrationPolicy = {
    enabled | Bool,
    profiles | Array SteelOrchestrationProfile | default = [],
  },

  TraceabilityRequirementSource = {
    root | SafeRelativePath,
    source_kind | TraceabilityRequirementSourceKind,
    extensions | NonEmptyArray | Array TraceabilityExtension,
  },

  TraceabilityEvidenceSource = {
    root | SafeRelativePath,
    source_kind | TraceabilityEvidenceSourceKind,
    extensions | NonEmptyArray | Array TraceabilityExtension,
  },

  TraceabilityProfile = {
    id | NonEmptyString,
    requirement_sources | NonEmptyArray | Array TraceabilityRequirementSource,
    evidence_sources | NonEmptyArray | Array TraceabilityEvidenceSource,
    marker_verbs | NonEmptyArray | Array TraceabilityMarkerVerb,
    assurance_level | TraceabilityAssuranceLevel,
    anchor_kind | TraceabilityAnchorKind,
  },

  TraceabilityPolicy = {
    default_profile | NonEmptyString,
    profiles | NonEmptyArray | Array TraceabilityProfile,
  },

  StackProvenancePolicy = {
    mode | StackProvenanceGateMode,
    allowed_claim_scopes | NonEmptyArray | Array NonEmptyString,
    accepted_valence_policy_hashes | Array Blake3HexContract | default = [],
    required_boundary | NonEmptyString,
  },

  FunctionAddressPolicy = {
    mode | StackProvenanceGateMode,
    allowed_claim_scopes | NonEmptyArray | Array NonEmptyString,
    accepted_valence_policy_hashes | Array Blake3HexContract | default = [],
    required_roles | NonEmptyArray | Array NonEmptyString,
    required_schemas | NonEmptyArray | Array NonEmptyString,
    required_boundary | NonEmptyString,
  },

  AuthenticatedPolicyExport = {
    evidence_kind | NonEmptyString,
    path | NonEmptyString,
    digest_blake3 | Blake3HexContract,
    signature_purpose | NonEmptyString,
  },

  AuthenticatedTrustedProducer = {
    producer | NonEmptyString,
    key_id | NonEmptyString,
    algorithm | NonEmptyString,
    public_key_hex | Blake3HexContract,
    allowed_purposes | NonEmptyArray | Array NonEmptyString,
    revoked | Bool,
  },

  AuthenticatedEvidencePolicy = {
    schema_version | NonEmptyString,
    release_profile | NonEmptyString,
    accepted_canonicalization_profiles | NonEmptyArray | Array NonEmptyString,
    accepted_valence_integrity_receipt_path | NonEmptyString,
    accepted_valence_integrity_receipt_hash | Blake3HexContract,
    policy_exports | NonEmptyArray | Array AuthenticatedPolicyExport,
    trusted_producers | Array AuthenticatedTrustedProducer | default = [],
  },

  WorkspaceContractMatrixGate = {
    mode | StackProvenanceGateMode,
    accepted_matrix_hashes | Array Blake3HexContract | default = [],
    required_boundary | NonEmptyString,
  },

  RuntimeEvidencePolicy = {
    allowed_capture_origins | NonEmptyArray | Array RuntimeCaptureOrigin,
    allowed_hash_algorithms | NonEmptyArray | Array Blake3Algorithm,
    max_summary_bytes | PositiveNumber,
    forbidden_summary_fragments | Array NonEmptyString | default = [],
    aggregation_profiles | Array NonEmptyString | default = [],
  },

  NominalIdentityDomain = {
    domain_id | NonEmptyString,
    category | NominalIdentityCategory,
    algorithm | NominalIdentityAlgorithm,
    required_prefix | String,
    max_bytes | PositiveNumber,
    rust_symbols | NonEmptyArray | Array NonEmptyString,
    core_scopes | NonEmptyArray | Array NonEmptyString,
    wire_compatibility_scopes | NonEmptyArray | Array NonEmptyString,
    raw_representation | NominalIdentityRawRepresentation,
    wrapper_kind | NominalIdentityWrapperKind,
    constructor_policy | NominalIdentityConstructorPolicy,
  },

  NominalIdentityRuntimeProjection = {
    schema_version | NonEmptyString,
    policy_id | NonEmptyString,
    domains | NonEmptyArray | Array NominalIdentityDomain,
  },

  NominalIdentityOctetDomain = {
    domain_id | NonEmptyString,
    symbols | NonEmptyArray | Array NonEmptyString,
    raw_representation | NominalIdentityRawRepresentation,
    core_scopes | NonEmptyArray | Array NonEmptyString,
    compatibility_scopes | NonEmptyArray | Array NonEmptyString,
    compatibility_required | Bool,
    wrapper_kind | NominalIdentityWrapperKind,
    constructor_policy | NominalIdentityConstructorPolicy,
  },

  NominalIdentityOctetProjection = {
    schema_version | NonEmptyString,
    policy_id | NonEmptyString,
    default_mode | NominalIdentityOctetMode,
    heuristic_suffixes | NonEmptyArray | Array NonEmptyString,
    domains | NonEmptyArray | Array NominalIdentityOctetDomain,
    non_claims | NonEmptyArray | Array NonEmptyString,
  },

  NominalIdentityPolicy = {
    policy_id | NonEmptyString,
    runtime | NominalIdentityRuntimeProjection,
    octet | NominalIdentityOctetProjection,
    non_claims | NonEmptyArray | Array NonEmptyString,
  },

  ProbeCommandProfile = {
    id | NonEmptyString,
    commands | NonEmptyArray | Array NonEmptyString,
    capability_classes | Array NonEmptyString | default = [],
  },

  ProbePolicy = {
    enabled | Bool,
    inline_gate_execution_enabled | Bool,
    receipt_schema_version | NonEmptyString,
    coverage_classes | NonEmptyArray | Array NonEmptyString,
    command_profiles | NonEmptyArray | Array ProbeCommandProfile,
    redaction_classes | NonEmptyArray | Array NonEmptyString,
  },

  ReceiptSchema = {
    command | NonEmptyString,
    top_level_keys | NonEmptyArray | Array NonEmptyString,
    action_keys | Array NonEmptyString | default = [],
    issue_keys | Array NonEmptyString | default = [],
    case_keys | Array NonEmptyString | default = [],
    mismatch_keys | Array NonEmptyString | default = [],
    artifact_keys | Array NonEmptyString | default = [],
    replay_evidence_keys | Array NonEmptyString | default = [],
    hash_fields | Array NonEmptyString | default = [],
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
    non_claims | NonEmptyArray | Array NonEmptyString,
  },

  ReplayCase = {
    id | NonEmptyString,
    command_kind | NonEmptyString,
    args | NonEmptyArray | Array NonEmptyString,
    fixture_root | NonEmptyString,
    expected_mutability | NonEmptyString,
    comparison_modes | NonEmptyArray | Array NonEmptyString,
    display_only_paths | Array NonEmptyString | default = [],
    hash_fields | Array NonEmptyString | default = [],
    normalization_rules | NonEmptyArray | Array NonEmptyString,
  },

  ReplayGroup = {
    id | NonEmptyString,
    description | NonEmptyString,
    cases | NonEmptyArray | Array NonEmptyString,
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
    required_replay_cases | NonEmptyArray | Array NonEmptyString,
    required_replay_groups | NonEmptyArray | Array NonEmptyString,
    determinism_claim | NonEmptyString,
  },

  ExemptionMarkerPolicy = {
    required_fields | NonEmptyArray | Array NonEmptyString,
    date_fields | NonEmptyArray | Array NonEmptyString,
    optional_fields | Array NonEmptyString | default = [],
  },

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

  PolicySchemaCompatibility = {
    schema_version | NonEmptyString,
    producer | NonEmptyString,
    field_set_digest | Blake3HexContract,
    refresh_command | NonEmptyString,
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

  CrossRepoDependencyPolicy = {
    schema | NonEmptyString,
    supported_receipt_kinds | NonEmptyArray | Array NonEmptyString,
    passing_status | NonEmptyString,
    non_passing_statuses | NonEmptyArray | Array NonEmptyString,
    max_evidence_file_bytes | PositiveNumber,
    max_evidence_files | PositiveNumber,
    required_non_claims | NonEmptyArray | Array NonEmptyString,
  },

  AgentCompletionPolicy = {
    schema | NonEmptyString,
    legacy_classification | NonEmptyString,
    passing_status | NonEmptyString,
    non_passing_statuses | NonEmptyArray | Array NonEmptyString,
    max_input_file_bytes | PositiveNumber,
    max_diagnostic_summary_bytes | PositiveNumber,
    max_reproduction_args | PositiveNumber,
    forbidden_diagnostic_fields | NonEmptyArray | Array NonEmptyString,
    required_non_claims | NonEmptyArray | Array NonEmptyString,
  },

  GlobalRegistryAdapterProfile = {
    id | NonEmptyString,
    kind | NonEmptyString,
    schema | NonEmptyString,
    capabilities | NonEmptyArray | Array NonEmptyString,
    record_families | NonEmptyArray | Array NonEmptyString,
  },

  GlobalRegistryTagNamespace = {
    id | NonEmptyString,
    authority | NonEmptyString,
  },

  GlobalRegistryGraphProfile = {
    id | NonEmptyString,
    allowed_node_kinds | NonEmptyArray | Array NonEmptyString,
    allowed_edge_kinds | NonEmptyArray | Array NonEmptyString,
    require_dag | Bool,
  },

  GlobalRegistryPolicy = {
    schema | NonEmptyString,
    storage_schema | NonEmptyString,
    max_artifact_bytes | PositiveNumber,
    adapters | NonEmptyArray | Array GlobalRegistryAdapterProfile,
    tag_namespaces | NonEmptyArray | Array GlobalRegistryTagNamespace,
    graph_profiles | NonEmptyArray | Array GlobalRegistryGraphProfile,
    allowed_vocabulary_owners | NonEmptyArray | Array NonEmptyString,
    required_non_claims | NonEmptyArray | Array NonEmptyString,
  },

  Policy = {
    name | NonEmptyString,
    version | SupportedPolicyVersion,
    policy_schema_compatibility | PolicySchemaCompatibility,
    schemas | NonEmptyArray | Array ArtifactSchema,
    workflow_profile_policy | WorkflowProfilePolicy,
    agent_guidance_policy | AgentGuidancePolicy,
    developer_loop_policy | DeveloperLoopPolicy,
    document_collaboration_policy | DocumentCollaborationPolicy,
    generated_agent_workflow_policy | GeneratedAgentWorkflowPolicy,
    lifecycle_stages | NonEmptyArray | Array Stage,
    task_marker_policy | TaskMarkerPolicy,
    gate_policy | GatePolicy,
    finding_defaults | FindingPolicy,
    report_contract | ReportContract,
    machine_contract_policy | MachineContractPolicy,
    registry_label_policy | RegistryLabelPolicy,
    change_metadata_policy | ChangeMetadataPolicy,
    cross_repo_dependency_policy | CrossRepoDependencyPolicy,
    agent_completion_policy | AgentCompletionPolicy,
    global_registry_policy | GlobalRegistryPolicy,
    lifecycle_store_policy | LifecycleStores.Policy,
    steel_orchestration_policy | SteelOrchestrationPolicy,
    traceability_policy | TraceabilityPolicy,
    stack_provenance_gate | StackProvenancePolicy,
    function_address_gate | FunctionAddressPolicy,
    authenticated_evidence_policy | AuthenticatedEvidencePolicy,
    workspace_contract_matrix_gate | WorkspaceContractMatrixGate | default = {
      mode = "disabled",
      accepted_matrix_hashes = [],
      required_boundary = "workspace contract matrix validation proves declared edge health only; it does not prove provider correctness, does not prove consumer behavioral correctness, does not prove verifier soundness, or does not prove release eligibility",
    },
    runtime_evidence_policy | RuntimeEvidencePolicy,
    nominal_identity_policy | NominalIdentityPolicy,
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
  },

  LifecycleStoreId = LifecycleStores.StoreId,
  LifecycleStoreRemoteHint = LifecycleStores.RemoteHint,
  LifecycleStoreReference = LifecycleStores.StoreReference,
  LifecycleStoreIdentitySource = LifecycleStores.StoreIdentitySource,
  LifecycleStoreIdentityProjection = LifecycleStores.StoreIdentityProjection,
  LifecycleStoreProjectSource = LifecycleStores.ProjectDeclarationSource,
  LifecycleStoreProjectProjection = LifecycleStores.ProjectDeclarationProjection,
  LifecycleStoreLocatorState = LifecycleStores.LocatorState,
  LifecycleStoreWorksetState = LifecycleStores.WorksetState,
  LifecycleStoreReceipt = LifecycleStores.LifecycleStoreReceipt,
  LifecycleStorePolicy = LifecycleStores.Policy,
  Blake3Hex = Blake3HexContract,
}
