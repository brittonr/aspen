let Contracts = import "../contracts.ncl" in
let Bounds = {
  max_artifacts = 4,
  max_dependencies = 4,
  max_files_per_artifact = 4,
  max_path_bytes = 256,
  max_artifact_bytes = 4096,
  max_template_bytes = 512,
  max_constraints = 4,
  max_non_claims = 4,
  max_issues = 16,
  max_directory_depth = 4,
  max_directory_entries = 16,
} in
(import "../default.ncl") & {
  schemas = [{
    schema_version = "cairn.workflow-schema.v1",
    id = "bad-template-schema",
    description = "invalid instruction template fixture",
    states = ["done", "ready", "blocked", "skipped"],
    bounds = Bounds,
    artifacts = [{
      id = "proposal",
      generates = "proposal.md",
      output_path = "proposal.md",
      path_kind = "file",
      structural_admission = "proposal",
      requires = [],
      stage = "proposal",
      instruction_template = "Write {{unknown}} for {{change}}.",
      constraints = [],
      apply_prerequisites = [],
      skipped = false,
      skip_satisfies_dependencies = false,
    }],
    required_non_claims = ["invalid template fixture proves nothing"],
  }],
} | Contracts.Policy
