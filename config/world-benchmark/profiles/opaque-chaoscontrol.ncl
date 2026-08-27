let Contracts = import "../contracts.ncl" in
let SourceRevision = "58185205ffc69ab3f7dc6cc04388885e7d43f562" in
let ProfileRef = "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee" in
let DatasetRef = "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff" in
let MemoryBytes = 134217728 in
let PageSizeBytes = 4096 in
let PageCount = 32768 in
let CopiedPages = 64 in
let CopiedBytes = 262144 in
let ObjectCount = 32768 in
let PreexistingObjects = 32704 in
let OperationCount = 1 in
let Repetitions = 3 in
let MaxPhysicalBytes = 268435456 in
let MaxObjects = 65536 in
let MaxPages = 65536 in
let MaxReferences = 65536 in
let MaxKeys = 1 in
let MaxConflicts = 0 in
let MaxDurationNanoseconds = 2000000000 in
let MaxPeakMemoryBytes = 536870912 in
{
  profile = {
    schema = "molten.world-benchmark-profile.v1",
    profile_ref = ProfileRef,
    source_revision = SourceRevision,
    dataset_ref = DatasetRef,
    preparation = "declared-warm",
    class = "opaque-exact-snapshot",
    adapters = ["chaoscontrol-exact-snapshot-descriptor-v1"],
    operations = ["snapshot-share"],
    bounds = {
      max_operations = OperationCount,
      max_repetitions = Repetitions,
      max_logical_bytes = MemoryBytes,
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
    hardware_cohort = "chaoscontrol-exact-x86-kvm-v1",
    thresholds = [
      {
        name = "snapshot-copied-page-ceiling",
        metric = "copied-pages",
        maximum = CopiedPages,
        operation = "snapshot-share",
      },
      {
        name = "snapshot-physical-write-ceiling",
        metric = "physical-bytes-written",
        maximum = CopiedBytes,
        operation = "snapshot-share",
      },
    ],
  },
  dataset = {
    dataset_ref = DatasetRef,
    source_revision = SourceRevision,
    shape = "downstream-shaped",
    preparation = "declared-warm",
    logical_bytes = MemoryBytes,
    object_count = ObjectCount,
    preexisting_objects = PreexistingObjects,
    changed_objects = CopiedPages,
    mutation_bytes = CopiedBytes,
    key_count = MaxKeys,
    page_count = PageCount,
    page_size_bytes = PageSizeBytes,
  },
} | Contracts.Input
