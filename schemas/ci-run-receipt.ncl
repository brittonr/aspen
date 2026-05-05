# Auto-generated Nickel contracts. Do not edit manually.
# Contract family: native-ci-run-receipt
# Source: crates/aspen-client-api/src/messages/ci.rs
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

let CiArtifactInfo = {
  blob_hash | String,
  name | String,
  size_bytes | Number,
  content_type | String,
  created_at | String,
  metadata | { _ : String },
} in

let CiRunReceiptJob = {
  name | String,
  job_id | String | optional,
  status | String,
  started_at_ms | Number | optional,
  completed_at_ms | Number | optional,
  error | String | optional,
  artifacts | Array CiArtifactInfo,
} in

let CiRunReceiptStage = {
  name | String,
  status | String,
  started_at_ms | Number | optional,
  completed_at_ms | Number | optional,
  jobs | Array CiRunReceiptJob,
} in

let CiRunReceipt = {
  schema | String,
  run_id | String,
  pipeline_name | String,
  repo_id | String,
  ref_name | String,
  commit_hash | String,
  status | String,
  created_at_ms | Number,
  started_at_ms | Number | optional,
  completed_at_ms | Number | optional,
  error | String | optional,
  stages | Array CiRunReceiptStage,
} in

{
  CiArtifactInfo,
  CiRunReceiptJob,
  CiRunReceiptStage,
  CiRunReceipt,
}
