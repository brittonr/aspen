# Aspen CI Pipeline Schema
# Nickel contracts for CI/CD configuration validation
#
# This schema defines the structure and validation rules for Aspen CI
# pipeline configuration. Contracts provide runtime type checking and
# helpful error messages when configuration is invalid.

# === Primitive Contracts ===

let NonEmptyString = std.contract.from_predicate (fun v =>
  std.is_string v && std.string.length v > 0
) in

let PositiveNumber = std.contract.from_predicate (fun v =>
  std.is_number v && v > 0
) in

let NonNegativeNumber = std.contract.from_predicate (fun v =>
  std.is_number v && v >= 0
) in

let DurationSeconds = std.contract.from_predicate (fun v =>
  std.is_number v && v >= 0 && v <= 86400 && v % 1 == 0
) in

let RetryCount = std.contract.from_predicate (fun v =>
  std.is_number v && v >= 0 && v <= 10 && v % 1 == 0
) in

let RetentionDays = std.contract.from_predicate (fun v =>
  std.is_number v && v >= 1 && v <= 365 && v % 1 == 0
) in

# === Enum Contracts ===

let JobType = [| 'nix, 'shell, 'vm |] in

let IsolationMode = [| 'nix_sandbox, 'vm, 'none |] in

let ArtifactStorage = [| 'blobs, 'local, 'none |] in

let Priority = [| 'high, 'normal, 'low |] in

# === Job Configuration ===

let JobConfig = {
  # Required: Job name (unique within stage)
  name | NonEmptyString,

  # Job type determines execution method
  type | JobType | default = 'shell,

  # Shell job fields
  command | String | optional,
  args | Array String | default = [],
  env | { _ : String } | default = {},
  working_dir | String | optional,

  # Nix job fields
  flake_url | String | optional,
  flake_attr | String | optional,

  # VM job fields
  binary_hash | String | optional,

  # Execution settings
  timeout_secs | DurationSeconds | default = 3600,
  isolation | IsolationMode | default = 'nix_sandbox,

  # Caching
  cache_key | String | optional,

  # Artifacts to collect (glob patterns)
  artifacts | Array String | default = [],

  # Job dependencies (names of other jobs in same stage)
  depends_on | Array String | default = [],

  # Retry behavior
  retry_count | RetryCount | default = 0,

  # If true, pipeline continues even if this job fails
  allow_failure | Bool | default = false,

  # Tags for worker affinity
  tags | Array String | default = [],
} in

# === Stage Configuration ===

let StageConfig = {
  # Required: Stage name (unique within pipeline)
  name | NonEmptyString,

  # Jobs to execute in this stage
  jobs | Array JobConfig,

  # If true, jobs run in parallel; if false, sequential
  parallel | Bool | default = true,

  # Stage dependencies (names of other stages)
  depends_on | Array String | default = [],

  # Only run this stage if ref matches pattern (e.g., "refs/heads/main")
  when | String | optional,
} in

# === Trigger Configuration ===

let TriggerConfig = {
  # Ref patterns to trigger on (e.g., ["refs/heads/main", "refs/heads/feature/*"])
  refs | Array String | default = ["refs/heads/main"],

  # Path patterns to ignore (changes to these paths won't trigger)
  ignore_paths | Array String | default = [],

  # Path patterns to watch (if set, only changes to these paths trigger)
  only_paths | Array String | default = [],
} in

# === Artifact Configuration ===

let ArtifactConfig = {
  # Where to store artifacts
  storage | ArtifactStorage | default = 'blobs,

  # How long to keep artifacts (days)
  retention_days | RetentionDays | default = 30,

  # Whether to compress artifacts
  compress | Bool | default = true,
} in

# === Pipeline Configuration ===

let PipelineConfig = {
  # Required: Pipeline name
  name | NonEmptyString,

  # Optional description
  description | String | optional,

  # Trigger configuration
  triggers | TriggerConfig | default = {},

  # Pipeline stages (executed in order, respecting dependencies)
  stages | Array StageConfig,

  # Global artifact settings
  artifacts | ArtifactConfig | default = {},

  # Global environment variables (available to all jobs)
  env | { _ : String } | default = {},

  # Pipeline-level timeout (seconds)
  timeout_secs | DurationSeconds | default = 7200,

  # Pipeline priority
  priority | Priority | default = 'normal,
} in

# Export all contracts
{
  PipelineConfig,
  StageConfig,
  JobConfig,
  TriggerConfig,
  ArtifactConfig,
  JobType,
  IsolationMode,
  ArtifactStorage,
  Priority,
  NonEmptyString,
  DurationSeconds,
  RetryCount,
  RetentionDays,
}
