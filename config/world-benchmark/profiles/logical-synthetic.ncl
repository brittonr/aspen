let Contracts = import "../contracts.ncl" in
let SourceRevision = "58185205ffc69ab3f7dc6cc04388885e7d43f562" in
let ProfileRef = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in
let DatasetRef = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" in
let LogicalBytes = 1048576 in
let ObjectCount = 128 in
let ChangedObjects = 8 in
let MutationBytes = 65536 in
let KeyCount = 256 in
let PageCount = 256 in
let PageSizeBytes = 4096 in
let OperationCount = 8 in
let Repetitions = 3 in
let MaxPhysicalBytes = 2097152 in
let MaxObjects = 512 in
let MaxPages = 512 in
let MaxReferences = 1024 in
let MaxKeys = 1024 in
let MaxConflicts = 64 in
let MaxDurationNanoseconds = 1000000000 in
let MaxPeakMemoryBytes = 67108864 in
{
  profile = {
    schema = "molten.world-benchmark-profile.v1",
    profile_ref = ProfileRef,
    source_revision = SourceRevision,
    dataset_ref = DatasetRef,
    preparation = "cold",
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
    hardware_cohort = "deterministic-count-fixture-v1",
    thresholds = [
      {
        name = "root-reference-write-ceiling",
        metric = "physical-bytes-written",
        maximum = LogicalBytes,
        operation = "root-branch",
      },
      {
        name = "replication-transfer-ceiling",
        metric = "transferred-bytes",
        maximum = MutationBytes,
        operation = "replication",
      },
    ],
  },
  dataset = {
    dataset_ref = DatasetRef,
    source_revision = SourceRevision,
    shape = "synthetic",
    preparation = "cold",
    logical_bytes = LogicalBytes,
    object_count = ObjectCount,
    preexisting_objects = 0,
    changed_objects = ChangedObjects,
    mutation_bytes = MutationBytes,
    key_count = KeyCount,
    page_count = PageCount,
    page_size_bytes = PageSizeBytes,
  },
} | Contracts.Input
