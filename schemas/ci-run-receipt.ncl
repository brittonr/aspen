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

let NonNegativeNumber = std.contract.from_validator (fun value =>
  if std.is_number value && value >= 0 then
    'Ok
  else
    'Error { message = "number must be non-negative" }
) in

let CiArtifactInfo = {
  blob_hash | String,
  name | String,
  size_bytes | NonNegativeNumber,
  content_type | String,
  created_at | String,
  metadata | { _ : String },
} in

let CiRunReceiptJob = {
  name | String,
  job_id | String | optional,
  status | String,
  started_at_ms | NonNegativeNumber | optional,
  completed_at_ms | NonNegativeNumber | optional,
  error | String | optional,
  artifacts | Array CiArtifactInfo,
} in

let CiRunReceiptStage = {
  name | String,
  status | String,
  started_at_ms | NonNegativeNumber | optional,
  completed_at_ms | NonNegativeNumber | optional,
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
  created_at_ms | NonNegativeNumber,
  started_at_ms | NonNegativeNumber | optional,
  completed_at_ms | NonNegativeNumber | optional,
  error | String | optional,
  stages | Array CiRunReceiptStage,
} in

let CiArtifactInfoContract = CiArtifactInfo in
let CiRunReceiptJobContract = CiRunReceiptJob in
let CiRunReceiptStageContract = CiRunReceiptStage in
let CiRunReceiptContract = CiRunReceipt in

{
  CiArtifactInfo = CiArtifactInfoContract,
  CiRunReceiptJob = CiRunReceiptJobContract,
  CiRunReceiptStage = CiRunReceiptStageContract,
  CiRunReceipt = CiRunReceiptContract,
}
