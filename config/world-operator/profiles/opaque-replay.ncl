let Contracts = import "../contracts.ncl" in
let ExpectedGeneration = 3 in
let MaximumOperations = 32 in
let MaximumDependencies = 32 in
let MaximumReceiptLinks = 64 in
let MaximumCanonicalBytes = 262144 in
let RequestRef = "blake3:589b75a935bb093a575482c6081c09d36046e24d70ed6a4943f0c5a0c81c09b6" in
let WorldRef = "blake3:76874bf8e2d46f80b9c53ab7ee2e9f2dc4f502765cd3f36f09864e61be311362" in
let ProfileRef = "blake3:c44f7cbbc1c6e7d7891c4685de9803c07068ad010c6a5322c5299bec1beb32a3" in
let Replay = "blake3:501f8eefa33fd4a96742ff83b8997d63278f6ec1c9da790a6ecfe68b8b82de26" in
{
  schema = "molten.world-workflow-request.v1",
  request_ref = RequestRef,
  world_ref = WorldRef,
  branch_id = "world/operator-opaque",
  expected_head = WorldRef,
  expected_generation = ExpectedGeneration,
  policy_ref = "blake3:ff096070fb25d5456f50000af78f1c92fda605bdb7bc3d1e7d1cc0091204e61c",
  authority_observation_ref = "blake3:aedae687c7102307927390b75e67062c68a1e099c4468bf2e7732e0dc46ad7a3",
  limits = {
    limits_ref = "blake3:a3f11a6fdc2c1e48e5b8bfa0c9023c73e4ec7a92d5aea981ea35faddd528fe1f",
    max_operations = MaximumOperations,
    max_dependencies_per_operation = MaximumDependencies,
    max_receipt_links = MaximumReceiptLinks,
    max_canonical_bytes = MaximumCanonicalBytes,
  },
  profiles = [{
    profile_ref = ProfileRef,
    kind = "opaque",
    status = "admitted",
    status_ref = "blake3:7c196f88a7e0220312a350117160f10b117b96b55420615b86939fab2d63f5de",
  }],
  observations = [
    {
      kind = "profile",
      observation_ref = "blake3:44bc86b91e3f55e4d0e5ea8917135712cb81ea877fb8499677907bd79eed8d61",
      subject_ref = ProfileRef,
      admitted = true,
    },
    {
      kind = "capsule-closure",
      observation_ref = "blake3:725387af70e79b377155c16b8ed191170aeb2a4c53689b47f14be8f8702cf496",
      subject_ref = WorldRef,
      admitted = true,
    },
  ],
  operations = [{
    operation_id = Replay,
    kind = "replay",
    subject_ref = WorldRef,
    profile_ref = ProfileRef,
    dependencies = [],
  }],
} | Contracts.Request
