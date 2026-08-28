let Contracts = import "../contracts.ncl" in
let ExpectedGeneration = 11 in
let MaximumOperations = 32 in
let MaximumDependencies = 32 in
let MaximumReceiptLinks = 64 in
let MaximumCanonicalBytes = 262144 in
let RequestRef = "blake3:154b69cbf0aaa75b1d1af2d159ed0593c3449a125287ed064c23bfd7cad0a0f9" in
let WorldRef = "blake3:d7894ae9716d38d2dfad0ec55424ca321ee12453d51f1b3adeb77d0475ed988c" in
let PolicyRef = "blake3:ff096070fb25d5456f50000af78f1c92fda605bdb7bc3d1e7d1cc0091204e61c" in
let AuthorityRef = "blake3:aedae687c7102307927390b75e67062c68a1e099c4468bf2e7732e0dc46ad7a3" in
let LimitsRef = "blake3:a3f11a6fdc2c1e48e5b8bfa0c9023c73e4ec7a92d5aea981ea35faddd528fe1f" in
let ProfileRef = "blake3:e83e8b548da9ac9932a5b86a421112e19e4b5344a1b169306174c5a696626ce7" in
let ProfileStatusRef = "blake3:8d861277e3d6cff3f5645e2ab0c2d858fd36b547e02717f376faf6cb56300bcf" in
let Inspect = "blake3:f219beffacee5f5597237fa7a6cb9ed03125fc1cd450a8f4817c69b7d82cdd60" in
let Checkpoint = "blake3:63741c69a919dad18d61f2f5a91477ac370eb1b3c70ee6bb45d842185eb6e5b3" in
let Branch = "blake3:8607a753672bd79841e5db55da0a5f151d165619ddba196aef9733dc8641fb7e" in
let Simulate = "blake3:6caeb7f4ebd66865bf5fb3463458609e04881603630d0696412abbd69b972653" in
let Run = "blake3:a7fceeda275f1d74d7dd05d0139905d286f7fe37b73b0ff295e35c83b36d8c42" in
let Diff = "blake3:44a0ab9468ec7f279b75bc2bb5e4493badafecff3eeea3c1627ef2870b542d9a" in
let Conflicts = "blake3:5fa1505ede5e82a3191be16693fa5464b5e3f9f63f19eb8739122591e975230c" in
let Replay = "blake3:5472fd1289bedd06ddc5bb5ebb4f6b4d9e288dc99668fa04bc45ae69b5102897" in
let Verify = "blake3:ca5f7a1f407a1a35a21511f3341a86f34aada14cb485a1ad96286228a9a946e6" in
let Promote = "blake3:5f5070a8d214fd0b54e7527547eb54ebc1a8846c102b3e6fcad81cdc248d95e0" in
let Export = "blake3:345ef650bb803ff61df8aaf9a3888d712fc141fe49bee1e6821c24597f95e0e4" in
let Import = "blake3:6b042a837f1d2dd20622c2847db98bc267711d7a2c5673f729a2940604065911" in
let GcPlan = "blake3:2a8556f9245297431e3ecd797c86cc8779a5a243b34ea7cf5b99acd7bc799fa2" in
{
  schema = "molten.world-workflow-request.v1",
  request_ref = RequestRef,
  world_ref = WorldRef,
  branch_id = "world/operator-dogfood",
  expected_head = WorldRef,
  expected_generation = ExpectedGeneration,
  policy_ref = PolicyRef,
  authority_observation_ref = AuthorityRef,
  limits = {
    limits_ref = LimitsRef,
    max_operations = MaximumOperations,
    max_dependencies_per_operation = MaximumDependencies,
    max_receipt_links = MaximumReceiptLinks,
    max_canonical_bytes = MaximumCanonicalBytes,
  },
  profiles = [{
    profile_ref = ProfileRef,
    kind = "logical",
    status = "admitted",
    status_ref = ProfileStatusRef,
  }],
  observations = [
    {
      kind = "profile",
      observation_ref = "blake3:44bc86b91e3f55e4d0e5ea8917135712cb81ea877fb8499677907bd79eed8d61",
      subject_ref = ProfileRef,
      admitted = true,
    },
    {
      kind = "head",
      observation_ref = "blake3:c0daa7fcaf2f4aae5a597e134c7ebc03c0ad0097be96e86c7575d461cf976368",
      subject_ref = WorldRef,
      admitted = true,
    },
    {
      kind = "policy",
      observation_ref = "blake3:f8c58d0140d06469141a5a92ecf975e889634fb3917d21190caa4e9ecfa8fb4e",
      subject_ref = PolicyRef,
      admitted = true,
    },
    {
      kind = "authority",
      observation_ref = AuthorityRef,
      subject_ref = WorldRef,
      admitted = true,
    },
    {
      kind = "conflict",
      observation_ref = "blake3:96df63278530d7c4193faa96dc24c203dba085998b253d86be87bc305f475329",
      subject_ref = WorldRef,
      admitted = true,
    },
    {
      kind = "effect",
      observation_ref = "blake3:933c62dbcd85b55f72f8da85c12dcd33a532a7ded2077c3383bfd6908797e1be",
      subject_ref = WorldRef,
      admitted = true,
    },
    {
      kind = "capsule-closure",
      observation_ref = "blake3:725387af70e79b377155c16b8ed191170aeb2a4c53689b47f14be8f8702cf496",
      subject_ref = WorldRef,
      admitted = true,
    },
    {
      kind = "retention",
      observation_ref = "blake3:3254e433ca9635a4a037939046e00e2fb48c5849e5a53c80b050b5b63e05a4af",
      subject_ref = WorldRef,
      admitted = true,
    },
  ],
  operations = [
    { operation_id = Inspect, kind = "inspect", subject_ref = WorldRef, profile_ref = ProfileRef, dependencies = [] },
    { operation_id = Checkpoint, kind = "checkpoint", subject_ref = WorldRef, profile_ref = ProfileRef, dependencies = [Inspect] },
    { operation_id = Branch, kind = "branch", subject_ref = WorldRef, profile_ref = ProfileRef, dependencies = [Checkpoint] },
    { operation_id = Simulate, kind = "simulate", subject_ref = WorldRef, profile_ref = ProfileRef, dependencies = [Branch] },
    { operation_id = Run, kind = "run", subject_ref = WorldRef, profile_ref = ProfileRef, dependencies = [Simulate] },
    { operation_id = Diff, kind = "diff", subject_ref = WorldRef, profile_ref = ProfileRef, dependencies = [Run] },
    { operation_id = Conflicts, kind = "conflicts", subject_ref = WorldRef, profile_ref = ProfileRef, dependencies = [Diff] },
    { operation_id = Replay, kind = "replay", subject_ref = WorldRef, profile_ref = ProfileRef, dependencies = [Conflicts] },
    { operation_id = Verify, kind = "verify", subject_ref = WorldRef, profile_ref = ProfileRef, dependencies = [Replay] },
    { operation_id = Promote, kind = "promote", subject_ref = WorldRef, profile_ref = ProfileRef, dependencies = [Verify] },
    { operation_id = Export, kind = "export", subject_ref = WorldRef, profile_ref = ProfileRef, dependencies = [Promote] },
    { operation_id = Import, kind = "import", subject_ref = WorldRef, profile_ref = ProfileRef, dependencies = [Export] },
    { operation_id = GcPlan, kind = "gc-plan", subject_ref = WorldRef, profile_ref = ProfileRef, dependencies = [Import] },
  ],
} | Contracts.Request
