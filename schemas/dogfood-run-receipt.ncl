# Auto-generated Nickel contracts. Do not edit manually.
# Contract family: dogfood-run-receipt
# Source: crates/aspen-dogfood/src/receipt.rs
# Regenerate with: python3 scripts/generate-typed-nickel-contracts.py --write

let contains = fun values value =>
  std.array.fold_left (fun found item => found || item == value) false values in

let enum_string = fun field allowed =>
  std.contract.from_validator (fun value =>
    if std.is_string value && contains allowed value then
      'Ok
    else
      'Error { message = "%{field} must be one of %{std.serialize 'Json allowed}" }
  ) in

let DogfoodRunMode = {
  federation | Bool,
  vm_ci | Bool,
} in

let DogfoodStageKind = enum_string "DogfoodStageKind" ["start", "push", "build", "deploy", "verify", "publish_receipt", "stop"] in

let DogfoodStageStatus = enum_string "DogfoodStageStatus" ["pending", "running", "succeeded", "failed", "skipped"] in

let DogfoodFailureSummary = {
  operation | String,
  category | String,
  message | String,
} in

let DogfoodArtifactKind = enum_string "DogfoodArtifactKind" ["cluster_ticket", "forge_repository", "git_commit", "ci_run", "ci_artifact", "deployment", "verification_report", "receipt", "log_excerpt"] in

let DogfoodArtifactReceipt = {
  name | String,
  kind | DogfoodArtifactKind,
  store_id | String | optional,
  blob_id | String | optional,
  digest | String | optional,
  size_bytes | Number | optional,
  relative_path | String | optional,
} in

let DogfoodStageReceipt = {
  stage | DogfoodStageKind,
  status | DogfoodStageStatus,
  started_at | String,
  finished_at | String | optional,
  elapsed_ms | Number | optional,
  failure | DogfoodFailureSummary | optional,
  artifacts | Array DogfoodArtifactReceipt,
} in

let DogfoodRunReceipt = {
  schema | String,
  run_id | String,
  command | String,
  created_at | String,
  mode | DogfoodRunMode,
  project_dir | String,
  cluster_dir | String,
  stages | Array DogfoodStageReceipt,
} in

{
  DogfoodRunMode,
  DogfoodStageKind,
  DogfoodStageStatus,
  DogfoodFailureSummary,
  DogfoodArtifactKind,
  DogfoodArtifactReceipt,
  DogfoodStageReceipt,
  DogfoodRunReceipt,
}
