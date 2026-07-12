# Typed contract for Molten's recorded-only Wasm component performance rail.
# Rust consumes the checked JSON export; it does not evaluate Nickel at runtime.

let Prelude = import "../nickel-contract-prelude.ncl" in
let SchemaId = "molten.wasm-component-performance-profile.v1" in
let SchemaVersion = 1 in
let SourceLanguage = "nickel" in
let ProfileId = "molten.wasm.performance.v1" in
let EvidenceRole = "recorded-only" in
let ComponentProfileId = "molten.wasm.component.v1" in
let SightglassRevision = "c18bbe75803a6a610f7ff3b15549c927c6e02667" in
let SightglassRunner = "sightglass-cli" in
let SightglassRawSchema = "sightglass-data.measurement.v1" in
let Measurement = "cycles" in
let RequiredPhases = ["compilation", "instantiation", "execution"] in

let FastBundleRefs = [
  "blake3:8984e68c835ce8bd0b3af729092b98cc2c47d3a7a8f2eada28f2d23970e26884",
] in
let DeepBundleRefs = [
  "blake3:6ea7c323251905bff8a51935567a0ce75fad233ac498530f202307c35ea9b925",
  "blake3:7393db08a46ff6733394426fa42c38c3a4be2ca11eaf8746631d0cae2fdc8a5b",
] in
let ActorWorkloadRef = "blake3:31e5f8b8a4579f62dde10521790a1a36905cb1900480fabb0e190f61b3f95a3b" in
let SystemExtensionWorkloadRef = "blake3:c41e25ce415096156dde58eb0057e60a3b84a6d4f1912506accc4eab1da39e26" in
let HostClassRef = "blake3:09959b9b17ddf92cef356c835fc8bfb6020f642d7f5018b9b758f0f8e9bfa62c" in
let ResourceEnvelopeRef = "blake3:291658cf3a3c5653c1f109b66453cef93e3f1b9e8a9bcb20d55014d2ad93ff1b" in
let EngineCohortRef = "blake3:5295b3f4ef5ec7f4ebabff7a2e8aaa8ef585a4dda6c40226b67988c5474c09d1" in
let EngineArtifactRef = "blake3:5295b3f4ef5ec7f4ebabff7a2e8aaa8ef585a4dda6c40226b67988c5474c09d1" in
let RunnerArtifactRef = "blake3:b06044cd1cc2067897ae836504de53e63e56b4171ec09180e08f9c4208927462" in

let PartsPerMillion = 1000000 in
let BasisPoints = 10000 in
let ConfidenceBasisPoints = 9500 in
let PracticalThresholdPpm = 10000 in
let MaxSampleValue = 1000000000 in
let BytesPerKiB = 1024 in
let KiBPerMiB = 1024 in
let MaxSightglassOutputMiB = 4 in
let MaxSightglassRunnerMiB = 32 in
let MaxSightglassEngineMiB = 64 in
let MaxSightglassBenchmarkMiB = 16 in
let MaxSightglassOutputBytes = MaxSightglassOutputMiB * KiBPerMiB * BytesPerKiB in
let MaxSightglassRunnerBytes = MaxSightglassRunnerMiB * KiBPerMiB * BytesPerKiB in
let MaxSightglassEngineBytes = MaxSightglassEngineMiB * KiBPerMiB * BytesPerKiB in
let MaxSightglassBenchmarkBytes = MaxSightglassBenchmarkMiB * KiBPerMiB * BytesPerKiB in
let MaxSightglassRunSeconds = 3600 in
let FastProcesses = 1 in
let FastIterations = 3 in
let DeepProcesses = 10 in
let DeepIterations = 10 in
let MaxSamplesPerPhase = 128 in
let MaxConcurrency = 16 in
let MaxQueueDepth = 64 in
let MaxPoolMemories = 32 in
let MaxPoolTables = 32 in
let ReviewedOptimizationProfileIds = [
  "molten.wasm.optimization.baseline.v1",
  "molten.wasm.optimization.pooling.v1",
  "molten.wasm.optimization.cow.v1",
  "molten.wasm.optimization.instance-pre.v1",
] in

let RequiredNonClaims = [
  "not-behavioral-correctness",
  "not-determinism-beyond-conformance",
  "not-authority",
  "not-security",
  "not-cross-machine-performance",
  "not-cross-runtime-ranking",
  "not-release-eligibility",
  "not-semantic-equivalence",
] in

let exact = fun expected actual => actual == expected in
let positive_integer = fun value => std.is_number value && value > 0 && value % 1 == 0 in
let valid_ref = fun value =>
  Prelude.is_non_empty_string value
  && std.string.is_match "^blake3:[0-9a-f]{64}$" value in
let valid_refs = fun values =>
  std.is_array values
  && std.array.all valid_ref values
  && Prelude.distinct_strings values in
let valid_sampling = fun value =>
  std.is_record value
  && positive_integer value.processes
  && positive_integer value.iterations_per_process
  && positive_integer value.min_samples_per_phase
  && positive_integer value.max_samples_per_phase
  && value.min_samples_per_phase == value.processes * value.iterations_per_process
  && value.max_samples_per_phase >= value.min_samples_per_phase
  && value.max_samples_per_phase <= MaxSamplesPerPhase in
let valid_suite = fun lane expected_bundles expected_workloads expected_processes expected_iterations value =>
  std.is_record value
  && exact lane value.lane
  && Prelude.is_non_empty_string value.suite_id
  && exact Measurement value.measurement
  && value.pin_to_single_core
  && valid_refs value.materialization_bundle_refs
  && value.materialization_bundle_refs == expected_bundles
  && valid_refs value.workload_refs
  && value.workload_refs == expected_workloads
  && exact HostClassRef value.host_class_ref
  && exact ResourceEnvelopeRef value.resource_envelope_ref
  && exact EngineCohortRef value.engine_cohort_ref
  && exact EngineArtifactRef value.engine_artifact_ref
  && exact RunnerArtifactRef value.runner_artifact_ref
  && value.phases == RequiredPhases
  && valid_sampling value.sampling
  && value.sampling.processes == expected_processes
  && value.sampling.iterations_per_process == expected_iterations in
let valid_optimization_limits = fun value =>
  std.is_record value
  && positive_integer value.max_concurrency
  && positive_integer value.max_queue_depth
  && positive_integer value.max_pool_memories
  && positive_integer value.max_pool_tables
  && value.max_concurrency == MaxConcurrency
  && value.max_queue_depth == MaxQueueDepth
  && value.max_pool_memories == MaxPoolMemories
  && value.max_pool_tables == MaxPoolTables
  && value.reviewed_profile_ids == ReviewedOptimizationProfileIds in
let valid_profile = fun value =>
  std.is_record value
  && exact ProfileId value.profile_id
  && exact EvidenceRole value.evidence_role
  && exact ComponentProfileId value.component_profile_id
  && std.is_record value.sightglass
  && exact SightglassRevision value.sightglass.revision
  && exact SightglassRunner value.sightglass.runner
  && exact SightglassRawSchema value.sightglass.raw_schema
  && value.phases == RequiredPhases
  && valid_suite "fast" FastBundleRefs [ActorWorkloadRef] FastProcesses FastIterations value.fast
  && valid_suite "deep" DeepBundleRefs [ActorWorkloadRef, SystemExtensionWorkloadRef] DeepProcesses DeepIterations value.deep
  && std.is_record value.comparison
  && value.comparison.parts_per_million == PartsPerMillion
  && value.comparison.basis_points == BasisPoints
  && value.comparison.confidence_basis_points == ConfidenceBasisPoints
  && value.comparison.practical_threshold_ppm == PracticalThresholdPpm
  && value.comparison.max_sample_value == MaxSampleValue
  && value.comparison.max_sightglass_output_bytes == MaxSightglassOutputBytes
  && value.comparison.max_sightglass_runner_bytes == MaxSightglassRunnerBytes
  && value.comparison.max_sightglass_engine_bytes == MaxSightglassEngineBytes
  && value.comparison.max_sightglass_benchmark_bytes == MaxSightglassBenchmarkBytes
  && value.comparison.max_sightglass_run_seconds == MaxSightglassRunSeconds
  && valid_optimization_limits value.optimization_limits
  && value.non_claims == RequiredNonClaims in
let valid_export = fun value =>
  std.is_record value
  && exact SchemaId value.schema_id
  && exact SchemaVersion value.schema_version
  && exact SourceLanguage value.source_language
  && valid_profile value.profile in

let ProfileContract = std.contract.from_predicate valid_profile in
let ExportContract = std.contract.from_predicate valid_export in

{
  schema_id = SchemaId,
  schema_version = SchemaVersion,
  source_language = SourceLanguage,
  profile_id = ProfileId,
  evidence_role = EvidenceRole,
  component_profile_id = ComponentProfileId,
  sightglass = {
    revision = SightglassRevision,
    runner = SightglassRunner,
    raw_schema = SightglassRawSchema,
  },
  phases = RequiredPhases,
  refs = {
    fast_bundles = FastBundleRefs,
    deep_bundles = DeepBundleRefs,
    actor_workload = ActorWorkloadRef,
    system_extension_workload = SystemExtensionWorkloadRef,
    host_class = HostClassRef,
    resource_envelope = ResourceEnvelopeRef,
    engine_cohort = EngineCohortRef,
    engine_artifact = EngineArtifactRef,
    runner_artifact = RunnerArtifactRef,
  },
  sampling = {
    fast = {
      processes = FastProcesses,
      iterations_per_process = FastIterations,
      max_samples_per_phase = FastProcesses * FastIterations,
    },
    deep = {
      processes = DeepProcesses,
      iterations_per_process = DeepIterations,
      max_samples_per_phase = MaxSamplesPerPhase,
    },
  },
  comparison = {
    parts_per_million = PartsPerMillion,
    basis_points = BasisPoints,
    confidence_basis_points = ConfidenceBasisPoints,
    practical_threshold_ppm = PracticalThresholdPpm,
    max_sample_value = MaxSampleValue,
    max_sightglass_output_bytes = MaxSightglassOutputBytes,
    max_sightglass_runner_bytes = MaxSightglassRunnerBytes,
    max_sightglass_engine_bytes = MaxSightglassEngineBytes,
    max_sightglass_benchmark_bytes = MaxSightglassBenchmarkBytes,
    max_sightglass_run_seconds = MaxSightglassRunSeconds,
  },
  optimization_limits = {
    max_concurrency = MaxConcurrency,
    max_queue_depth = MaxQueueDepth,
    max_pool_memories = MaxPoolMemories,
    max_pool_tables = MaxPoolTables,
    reviewed_profile_ids = ReviewedOptimizationProfileIds,
  },
  non_claims = RequiredNonClaims,
  Profile = ProfileContract,
  Export = ExportContract,
}
