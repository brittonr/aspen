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
let Artifact = fun id path requires => {
  id = id,
  generates = path,
  output_path = path,
  path_kind = "file",
  structural_admission = "markdown",
  requires = requires,
  stage = "design",
  instruction_template = "Write {{artifact}} for {{change}}.",
  constraints = [],
  apply_prerequisites = [],
  skipped = false,
  skip_satisfies_dependencies = false,
} in
(import "../default.ncl") & {
  schemas = [{
    schema_version = "cairn.workflow-schema.v1",
    id = "cyclic-schema",
    description = "cyclic artifact dependency fixture",
    states = ["done", "ready", "blocked", "skipped"],
    bounds = Bounds,
    artifacts = [
      Artifact "first" "first.md" ["second"],
      Artifact "second" "second.md" ["first"],
    ],
    required_non_claims = ["cyclic fixture proves nothing"],
  }],
} | Contracts.Policy
