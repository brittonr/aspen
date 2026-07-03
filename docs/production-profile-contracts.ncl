# Shared Nickel contracts and reviewed constants for production node profiles.
# Runtime Rust consumes checked exported JSON and receipts; it does not evaluate Nickel.

let SchemaId = "molten.prod-ops.deployment-profile.v1" in
let SchemaVersion = 1 in
let SourceLanguage = "nickel" in

let BytesPerKiB = 1024 in
let KiB = BytesPerKiB in
let MiB = KiB * KiB in
let GiB = KiB * MiB in
let MillisecondsPerSecond = 1000 in
let SecondsPerMinute = 60 in

let ProductionQueueDepth = 1024 in
let MaxReceiptBytes = 128 * MiB in
let MaxStoreBytes = 10 * GiB in
let MaxDeliveryLatencyMs = 5 * MillisecondsPerSecond in
let MaxRecoveryTimeMs = SecondsPerMinute * MillisecondsPerSecond in

let RequiredProductionAdapters = [
  "node-control",
  "evidence-ledger",
  "chunk-store",
  "retention-index",
  "live-iroh",
] in

let RedactionSettings = [
  "deny-sensitive-export-by-default",
  "diagnostic-logs-auxiliary-only",
  "private-ref-reveal-requires-receipt",
] in

let LiveTransportSettings = [
  "node-control-topic-only",
  "ticket-bound-peer-admission",
  "authority-grant-ledger-required",
] in

let StartupExpectations = [
  "profile-ref-bound-in-startup-receipt",
  "source-gate-current-before-run",
  "required-adapters-start-in-deterministic-order",
] in

let ShutdownExpectations = [
  "bounded-drain-before-stop",
  "shutdown-receipt-imported",
  "active-lock-removed-after-pass",
] in

let is_non_empty_string = fun value =>
  std.is_string value && std.string.length value > 0 in

let is_blake3_ref = fun value =>
  std.is_string value && std.string.is_match "^blake3:[0-9a-f]{64}$" value in

let is_absolute_state_root = fun value =>
  std.is_string value
  && std.string.length value > 1
  && std.string.is_match "^/[A-Za-z0-9._/-]+$" value
  && !std.string.is_match "(^|/)\\.\\.(/|$)" value
  && !std.string.is_match "(^|/)\\.git(/|$)" value in

let is_safe_relative_dir = fun value =>
  std.is_string value
  && std.string.length value > 0
  && std.string.is_match "^[A-Za-z0-9._-]+$" value
  && value != "."
  && value != ".."
  && !std.string.is_match "^\\." value in

let is_positive_integer = fun value =>
  std.is_number value && std.number.is_integer value && value > 0 in

let is_exact_schema_id = fun value =>
  std.is_string value && value == SchemaId in

let is_exact_schema_version = fun value =>
  std.is_number value && std.number.is_integer value && value == SchemaVersion in

let is_exact_source_language = fun value =>
  std.is_string value && value == SourceLanguage in

let is_one_of = fun allowed value =>
  std.is_string value && std.array.elem value allowed in

let has_all = fun actual required =>
  std.array.all (fun item => std.array.elem item actual) required in

let distinct_strings = fun values =>
  std.array.length values == std.array.length (std.array.sort_dedup std.string.compare values) in

let is_non_empty_array = fun values =>
  std.is_array values && std.array.length values > 0 in

let layout_dirs = fun layout => [
  layout.ledger_dir,
  layout.redb_dir,
  layout.chunk_dir,
  layout.identity_dir,
  layout.retention_dir,
  layout.inbox_dir,
] in

let is_state_layout = fun layout =>
  std.is_record layout
  && is_absolute_state_root layout.state_root
  && is_safe_relative_dir layout.ledger_dir
  && is_safe_relative_dir layout.redb_dir
  && is_safe_relative_dir layout.chunk_dir
  && is_safe_relative_dir layout.identity_dir
  && is_safe_relative_dir layout.retention_dir
  && is_safe_relative_dir layout.inbox_dir
  && distinct_strings (layout_dirs layout) in

let is_resource_limits = fun limits =>
  std.is_record limits
  && is_positive_integer limits.max_queue_depth
  && is_positive_integer limits.max_receipt_bytes
  && is_positive_integer limits.max_store_bytes
  && is_positive_integer limits.max_delivery_latency_ms
  && is_positive_integer limits.max_recovery_time_ms
  && limits.max_store_bytes >= limits.max_receipt_bytes
  && limits.max_recovery_time_ms >= limits.max_delivery_latency_ms in

let array_of = fun predicate values =>
  is_non_empty_array values && std.array.all predicate values in

let is_production_node_profile = fun profile =>
  std.is_record profile
  && is_non_empty_string profile.profile_name
  && is_state_layout profile.state_layout
  && array_of (is_one_of RequiredProductionAdapters) profile.required_adapters
  && has_all profile.required_adapters RequiredProductionAdapters
  && array_of is_blake3_ref profile.source_gate_inputs
  && is_resource_limits profile.resource_limits
  && array_of (is_one_of RedactionSettings) profile.redaction_settings
  && array_of (is_one_of LiveTransportSettings) profile.live_transport_settings
  && array_of (is_one_of StartupExpectations) profile.startup_expectations
  && array_of (is_one_of ShutdownExpectations) profile.shutdown_expectations in

let is_production_profile_export = fun value =>
  std.is_record value
  && is_exact_schema_id value.schema_id
  && is_exact_schema_version value.schema_version
  && is_exact_source_language value.source_language
  && is_non_empty_string value.profile_identity
  && is_production_node_profile value.profile
  && value.profile_identity == value.profile.profile_name in

let NonEmptyStringContract = std.contract.from_predicate is_non_empty_string in
let Blake3RefContract = std.contract.from_predicate is_blake3_ref in
let AbsoluteStateRootContract = std.contract.from_predicate is_absolute_state_root in
let SafeRelativeDirContract = std.contract.from_predicate is_safe_relative_dir in
let PositiveIntegerContract = std.contract.from_predicate is_positive_integer in
let ExactSchemaIdContract = std.contract.from_predicate is_exact_schema_id in
let ExactSchemaVersionContract = std.contract.from_predicate is_exact_schema_version in
let ExactSourceLanguageContract = std.contract.from_predicate is_exact_source_language in
let AdapterNameContract = std.contract.from_predicate (is_one_of RequiredProductionAdapters) in
let RedactionSettingContract = std.contract.from_predicate (is_one_of RedactionSettings) in
let LiveTransportSettingContract = std.contract.from_predicate (is_one_of LiveTransportSettings) in
let StartupExpectationContract = std.contract.from_predicate (is_one_of StartupExpectations) in
let ShutdownExpectationContract = std.contract.from_predicate (is_one_of ShutdownExpectations) in
let StateLayoutContract = std.contract.from_predicate is_state_layout in
let ResourceLimitsContract = std.contract.from_predicate is_resource_limits in
let ProductionNodeProfileContract = std.contract.from_predicate is_production_node_profile in
let ProductionProfileExportContract = std.contract.from_predicate is_production_profile_export in

{
  schema_id = SchemaId,
  schema_version = SchemaVersion,
  source_language = SourceLanguage,
  units = {
    bytes_per_kib = BytesPerKiB,
    kib = KiB,
    mib = MiB,
    gib = GiB,
    milliseconds_per_second = MillisecondsPerSecond,
    seconds_per_minute = SecondsPerMinute,
  },
  thresholds = {
    max_queue_depth = ProductionQueueDepth,
    max_receipt_bytes = MaxReceiptBytes,
    max_store_bytes = MaxStoreBytes,
    max_delivery_latency_ms = MaxDeliveryLatencyMs,
    max_recovery_time_ms = MaxRecoveryTimeMs,
  },
  required_adapters = RequiredProductionAdapters,
  vocabulary = {
    redaction_settings = RedactionSettings,
    live_transport_settings = LiveTransportSettings,
    startup_expectations = StartupExpectations,
    shutdown_expectations = ShutdownExpectations,
  },
  contracts = {
    AbsoluteStateRoot = AbsoluteStateRootContract,
    AdapterName = AdapterNameContract,
    Blake3Ref = Blake3RefContract,
    ExactSchemaId = ExactSchemaIdContract,
    ExactSchemaVersion = ExactSchemaVersionContract,
    ExactSourceLanguage = ExactSourceLanguageContract,
    LiveTransportSetting = LiveTransportSettingContract,
    NonEmptyString = NonEmptyStringContract,
    PositiveInteger = PositiveIntegerContract,
    ProductionNodeProfile = ProductionNodeProfileContract,
    RedactionSetting = RedactionSettingContract,
    ResourceLimits = ResourceLimitsContract,
    SafeRelativeDir = SafeRelativeDirContract,
    ShutdownExpectation = ShutdownExpectationContract,
    StartupExpectation = StartupExpectationContract,
    StateLayout = StateLayoutContract,
  },
  ProductionProfileExport = ProductionProfileExportContract,
}
