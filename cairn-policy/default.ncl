# Runtime compatibility migration from the committed 9ae893f31 policy projection.
# The old authored file lagged that projection (trust hashes, replay cases, and
# receipt fields). Select each preserved value explicitly; never reset policy.
let Contracts = import "contracts.ncl" in
let Runtime = import "vendor/cairn-15f00875/cairn-policy/default.ncl" in
{
  name = Runtime.name,
  version = Runtime.version,
  policy_schema_compatibility = Runtime.policy_schema_compatibility,
  schemas = Runtime.schemas,
  workflow_profile_policy = Runtime.workflow_profile_policy,
  agent_guidance_policy = Runtime.agent_guidance_policy,
  developer_loop_policy = Runtime.developer_loop_policy,
  document_collaboration_policy = Runtime.document_collaboration_policy,
  generated_agent_workflow_policy = Runtime.generated_agent_workflow_policy,
  lifecycle_stages = Runtime.lifecycle_stages,
  task_marker_policy = Runtime.task_marker_policy,
  gate_policy = Runtime.gate_policy,
  finding_defaults = Runtime.finding_defaults,
  report_contract = Runtime.report_contract,
  machine_contract_policy = Runtime.machine_contract_policy,
  registry_label_policy = Runtime.registry_label_policy,
  change_metadata_policy = Runtime.change_metadata_policy,
  cross_repo_dependency_policy = Runtime.cross_repo_dependency_policy,
  agent_completion_policy = Runtime.agent_completion_policy,
  global_registry_policy = Runtime.global_registry_policy,
  lifecycle_store_policy = Runtime.lifecycle_store_policy,
  steel_orchestration_policy = Runtime.steel_orchestration_policy,
  traceability_policy = {
    default_profile = "cairn-default",
    profiles = [
      {
        id = "cairn-default",
        assurance_level = "declared",
        anchor_kind = "exact_marker_bytes",
        requirement_sources = [{ root = ".cairn/specs", source_kind = "accepted_specs", extensions = ["md"] }],
        evidence_sources = [
          { root = "src", source_kind = "rust_source", extensions = ["rs"] },
          { root = "crates", source_kind = "rust_source", extensions = ["rs"] },
          { root = "crates", source_kind = "nickel_source", extensions = ["ncl"] },
          { root = "tools", source_kind = "tool_source", extensions = ["rs", "ncl"] },
        ],
        marker_verbs = ["impl", "verify", "depends", "related"],
      },
      {
        id = "trellis-rust",
        assurance_level = "declared",
        anchor_kind = "exact_marker_bytes",
        requirement_sources = [{ root = "docs/requirements.md", source_kind = "requirements_doc", extensions = ["md"] }],
        evidence_sources = [
          { root = "src", source_kind = "rust_source", extensions = ["rs"] },
          { root = "tests", source_kind = "rust_source", extensions = ["rs"] },
        ],
        marker_verbs = ["impl", "verify", "depends", "related"],
      },
    ],
  },
  stack_provenance_gate = Runtime.stack_provenance_gate,
  function_address_gate = Runtime.function_address_gate,
  workspace_contract_matrix_gate = Runtime.workspace_contract_matrix_gate,
  authenticated_evidence_policy = Runtime.authenticated_evidence_policy,
  runtime_evidence_policy = Runtime.runtime_evidence_policy,
  nominal_identity_policy = Runtime.nominal_identity_policy,
  structured_lifecycle = Runtime.structured_lifecycle,
  probe_policy = Runtime.probe_policy,
  # Keep the exact committed gate receipt surface for the selected CLI cohort.
  receipt_schemas =
    (
      std.array.map
        (fun entry =>
          if entry.command == "gate" then
            entry
            & {
              top_level_keys | force = std.array.filter (fun key => key != "task_ordering") entry.top_level_keys,
            }
          else
            entry
        )
        Runtime.receipt_schemas
    )
    @ (import "retained-receipt-contracts.ncl"),
  receipt_contracts = Runtime.receipt_contracts,
  determinism_replay_policy = Runtime.determinism_replay_policy,
  determinism_surface_inventory = Runtime.determinism_surface_inventory,
  sync_archive_policy = Runtime.sync_archive_policy,
  mutation_policy = Runtime.mutation_policy,
  receipt_hash_policy = Runtime.receipt_hash_policy,
  exemption_marker_policy = Runtime.exemption_marker_policy,
  exemptions = Runtime.exemptions,
} | Contracts.Policy
