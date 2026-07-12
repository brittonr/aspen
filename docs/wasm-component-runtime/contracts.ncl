# Typed compatibility contract for Molten's first production Component Model cohort.
# Rust consumes the checked JSON export; it does not evaluate Nickel at runtime.

let Prelude = import "../nickel-contract-prelude.ncl" in
let SchemaId = "molten.wasm-component-profile.v1" in
let SchemaVersion = 1 in
let SourceLanguage = "nickel" in
let ProfileId = "molten.wasm.component.v1" in
let EvidenceScope = "production" in
let RuntimeStrategy = "portable-component" in
let WasmtimeVersion = "45.0.0" in
let WasmToolsVersion = "1.240.0" in
let WasmparserVersion = "0.240.0" in
let WitBindgenVersion = "0.46.0" in
let WasmtimeWasiVersion = "45.0.0" in
let WasiPackageVersion = "0.2.6" in
let WitPackage = "molten:component-runtime@1.0.0" in
let WitWorld = "actor" in
let WitSourceRef = "blake3:83be349bb27975ada30dbe60817c5404df7862babdf02ae229b399886e76d5e8" in

let BytesPerKiB = 1024 in
let KiB = BytesPerKiB in
let MiB = KiB * KiB in
let ComponentFuel = 100000 in
let MaxComponentBytes = 4 * MiB in
let MaxWitBytes = 64 * KiB in
let MaxMemoryBytes = 16 * MiB in
let MaxTableElements = 1024 in
let MaxInstances = 16 in
let MaxMemories = 4 in
let MaxTables = 4 in
let MaxStackBytes = 512 * KiB in
let MaxHostcallBytes = 64 * KiB in
let MaxResultBytes = 64 * KiB in
let MaxConcurrency = 1 in
let MaxImports = 128 in
let MaxExports = 128 in

let RequiredNonClaims = [
  "not-behavioral-correctness",
  "not-semantic-equivalence",
  "not-authority",
  "not-release-eligibility",
  "not-whole-system-safety",
  "not-build-trust-transfer",
] in

let exact = fun expected actual => actual == expected in
let positive_integer = fun value => std.is_number value && value > 0 && value % 1 == 0 in
let string_array = Prelude.optional_array_of Prelude.is_non_empty_string in
let distinct = Prelude.distinct_strings in

let valid_toolchain = fun value =>
  std.is_record value
  && exact WasmtimeVersion value.wasmtime
  && exact WasmToolsVersion value.wasm_tools
  && exact WasmparserVersion value.wasmparser
  && exact WitBindgenVersion value.wit_bindgen
  && exact WasmtimeWasiVersion value.wasmtime_wasi
  && exact WasiPackageVersion value.wasi_package in

let valid_wit = fun value =>
  std.is_record value
  && exact WitPackage value.package
  && exact WitWorld value.world
  && exact WitSourceRef value.source_ref in

let valid_features = fun value =>
  std.is_record value
  && value.component_model
  && value.multi_value
  && value.bulk_memory
  && value.reference_types
  && value.simd
  && !value.relaxed_simd
  && !value.threads
  && !value.tail_call
  && !value.multi_memory
  && !value.exceptions
  && !value.gc
  && !value.memory64
  && !value.extended_const
  && !value.function_references
  && !value.custom_page_sizes
  && !value.wide_arithmetic
  && !value.component_async in

let valid_determinism = fun value =>
  std.is_record value
  && value.fuel_interruption
  && value.nan_canonicalization
  && value.relaxed_simd_deterministic
  && exact "fixed" value.memory_growth
  && exact "fixed" value.table_growth
  && exact "recorded-effects-only" value.host_inputs in

let valid_resources = fun value =>
  std.is_record value
  && positive_integer value.fuel
  && positive_integer value.max_component_bytes
  && positive_integer value.max_wit_bytes
  && positive_integer value.max_memory_bytes
  && positive_integer value.max_table_elements
  && positive_integer value.max_instances
  && positive_integer value.max_memories
  && positive_integer value.max_tables
  && positive_integer value.max_stack_bytes
  && positive_integer value.max_hostcall_bytes
  && positive_integer value.max_result_bytes
  && positive_integer value.max_concurrency
  && positive_integer value.max_imports
  && positive_integer value.max_exports
  && value.fuel == ComponentFuel
  && value.max_component_bytes == MaxComponentBytes
  && value.max_wit_bytes == MaxWitBytes
  && value.max_memory_bytes == MaxMemoryBytes
  && value.max_table_elements == MaxTableElements
  && value.max_instances == MaxInstances
  && value.max_memories == MaxMemories
  && value.max_tables == MaxTables
  && value.max_stack_bytes == MaxStackBytes
  && value.max_hostcall_bytes == MaxHostcallBytes
  && value.max_result_bytes == MaxResultBytes
  && value.max_concurrency == MaxConcurrency
  && value.max_imports == MaxImports
  && value.max_exports == MaxExports in

let valid_profile = fun value =>
  std.is_record value
  && exact ProfileId value.profile_id
  && exact EvidenceScope value.evidence_scope
  && exact RuntimeStrategy value.runtime_strategy
  && valid_toolchain value.toolchain
  && valid_wit value.wit
  && valid_features value.features
  && valid_determinism value.determinism
  && valid_resources value.resources
  && string_array value.allowed_imports
  && distinct value.allowed_imports
  && std.array.length value.allowed_imports == 0
  && string_array value.allowed_wasi_interfaces
  && distinct value.allowed_wasi_interfaces
  && std.array.length value.allowed_wasi_interfaces == 0
  && string_array value.non_claims
  && distinct value.non_claims
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
  evidence_scope = EvidenceScope,
  runtime_strategy = RuntimeStrategy,
  toolchain = {
    wasmtime = WasmtimeVersion,
    wasm_tools = WasmToolsVersion,
    wasmparser = WasmparserVersion,
    wit_bindgen = WitBindgenVersion,
    wasmtime_wasi = WasmtimeWasiVersion,
    wasi_package = WasiPackageVersion,
  },
  wit = {
    package = WitPackage,
    world = WitWorld,
    source_ref = WitSourceRef,
  },
  resources = {
    fuel = ComponentFuel,
    max_component_bytes = MaxComponentBytes,
    max_wit_bytes = MaxWitBytes,
    max_memory_bytes = MaxMemoryBytes,
    max_table_elements = MaxTableElements,
    max_instances = MaxInstances,
    max_memories = MaxMemories,
    max_tables = MaxTables,
    max_stack_bytes = MaxStackBytes,
    max_hostcall_bytes = MaxHostcallBytes,
    max_result_bytes = MaxResultBytes,
    max_concurrency = MaxConcurrency,
    max_imports = MaxImports,
    max_exports = MaxExports,
  },
  non_claims = RequiredNonClaims,
  Profile = ProfileContract,
  Export = ExportContract,
}
