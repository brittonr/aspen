let Contracts = import "../contracts.ncl" in
let SourceRevision = "58185205ffc69ab3f7dc6cc04388885e7d43f562" in
let ProfileRef = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc" in
let DatasetRef = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd" in
let LogicalBytes = 8388608 in
let ObjectCount = 1024 in
let PreexistingObjects = 1008 in
let ChangedObjects = 16 in
let MutationBytes = 131072 in
let KeyCount = 4096 in
let PageCount = 2048 in
let PageSizeBytes = 4096 in
let OperationCount = 8 in
let Repetitions = 3 in
let MaxPhysicalBytes = 16777216 in
let MaxObjects = 4096 in
let MaxPages = 4096 in
let MaxReferences = 8192 in
let MaxKeys = 8192 in
let MaxConflicts = 256 in
let MaxDurationNanoseconds = 2000000000 in
let MaxPeakMemoryBytes = 268435456 in
{
  profile = {
    schema = "molten.world-benchmark-profile.v1",
    profile_ref = ProfileRef,
    source_revision = SourceRevision,
    dataset_ref = DatasetRef,
    preparation = "declared-warm",
    class = "logical",
    adapters = ["molten-deterministic-world-benchmark-v1"],
    operations = [
      "root-branch",
      "first-mutation",
      "repeated-mutation",
      "diff",
      "merge-plan",
      "capsule-export",
      "replication",
      "retention-plan",
    ],
    bounds = {
      max_operations = OperationCount,
      max_repetitions = Repetitions,
      max_logical_bytes = LogicalBytes,
      max_physical_bytes = MaxPhysicalBytes,
      max_objects = MaxObjects,
      max_pages = MaxPages,
      max_references = MaxReferences,
      max_keys = MaxKeys,
      max_conflicts = MaxConflicts,
      max_duration_nanoseconds = MaxDurationNanoseconds,
      max_peak_memory_bytes = MaxPeakMemoryBytes,
    },
    repetitions = Repetitions,
    hardware_cohort = "downstream-shaped-count-fixture-v1",
    thresholds = [
      {
        name = "mutation-write-ceiling",
        metric = "physical-bytes-written",
        maximum = MutationBytes,
        operation = "first-mutation",
      },
      {
        name = "retention-candidate-ceiling",
        metric = "planned-deletions",
        maximum = 0,
        operation = "retention-plan",
      },
    ],
  },
  dataset = {
    dataset_ref = DatasetRef,
    source_revision = SourceRevision,
    shape = "downstream-shaped",
    preparation = "declared-warm",
    logical_bytes = LogicalBytes,
    object_count = ObjectCount,
    preexisting_objects = PreexistingObjects,
    changed_objects = ChangedObjects,
    mutation_bytes = MutationBytes,
    key_count = KeyCount,
    page_count = PageCount,
    page_size_bytes = PageSizeBytes,
  },
} | Contracts.Input
