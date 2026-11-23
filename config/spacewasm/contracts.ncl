let P = import "../../docs/nickel-contract-prelude.ncl" in
let Exact = fun v => P.contract (P.is_exact_string v) in
let Bound = fun maximum => P.contract (fun v => P.is_positive_integer v && v <= maximum) in
let NonClaims = ["not-component-compatible", "not-portable-state", "not-cross-host-migration",
  "not-runtime-correctness", "not-sandbox-completeness", "not-production-ready", "not-release-eligible"] in
let LimitsContract = {
  allocation_bytes | Bound 2147483647,
  ir_bytes | Bound 65536,
  stack_bytes | Bound 2147483647 | P.contract (fun v => std.number.is_integer (v / 4)),
  memory_pages | Bound 256,
  table_elements | P.contract (P.is_exact_integer 0),
  segment_fuel | Bound 9007199254740991,
  total_fuel | Bound 9007199254740991,
  hostcall_bytes | Bound 16777216,
  input_bytes | Bound 16777216,
  output_bytes | Bound 16777216,
  continuation_count | Bound 65536,
  continuation_bytes | Bound 9007199254740991,
  retention_turns | Bound 9007199254740991,
} in
let CohortContract = {
  module_features | Exact "wasm1",
  materialization_schema | Exact "mantle-spacewasm-reference-bundle-v1",
  materialization_profile | Exact "nasa-spacewasm-e24cf093-reference-v1",
  octet_receipt_schema | Exact "octet-evidence-rail-receipt/v1",
  octet_profile | Exact "spacewasm-molten-abi-pilot",
  octet_runtime_cohort | Exact "b3:7c537eef7bff66d0a90508715de08de79c9aaac23352e1007a6d47e7fb4674a2",
  chaos_differential_schema | Exact "chaoscontrol.spacewasm-molten-abi-differential.v1",
  reference_engine | Exact "wasmtime",
  reference_engine_version | Exact "49.0.5",
} in
let ProfileContract = {
  name | Exact "spacewasm-core-mvp-pilot",
  source_revision | Exact "e24cf09355a90497148eb5029fdb8e3400bd63e3",
  abi | Exact "molten.wasm.abi.v1",
  cohort | CohortContract,
  limits | LimitsContract | P.contract (fun v => v.segment_fuel <= v.total_fuel
    && v.ir_bytes <= v.allocation_bytes && v.stack_bytes <= v.allocation_bytes
    && v.hostcall_bytes <= v.memory_pages * 65536),
  production | P.contract (fun v => v == false),
  release | P.contract (fun v => v == false),
  non_claims | P.contract (fun v => v == NonClaims),
} in
let HexDigest = P.contract (fun value => std.is_string value
  && std.string.is_match "^[0-9a-f]{64}$" value) in
let OctetIdentity = P.contract (fun value => std.is_string value
  && std.string.is_match "^b3:[0-9a-f]{64}$" value) in
let Relative = P.contract (fun value => P.is_non_empty_string value
  && std.array.all P.is_safe_relative_dir (std.string.split "/" value)) in
let Verifier = {
  path | P.contract (fun value => std.is_string value
    && std.string.is_match "^/nix/store/[0-9abcdfghijklmnpqrsvwxyz]{32}-[A-Za-z0-9._+?=-]+(/[A-Za-z0-9._+?=-]+)*$" value
    && !std.string.is_match "(^|/)\\.\\.?(/|$)" value),
  blake3 | HexDigest,
} in
let Cohort = {
  mantle | Verifier,
  octet | Verifier,
  chaos | Verifier,
  bundle_identity | HexDigest,
  octet_profile_identity | OctetIdentity,
  cohort_identity | HexDigest,
  probe_blake3 | HexDigest,
} in
let Scenario = std.contract.any_of [
  P.contract (P.is_one_of ["Standard", "StaleGeneration", "HostPause"]),
  { HostTrap | { reason | String } },
  { HostUncertain | { reason | String } },
  { Cancel | { reason | String, after_segments | P.contract (fun value =>
      P.is_non_negative_integer value && value <= 9007199254740991) } },
] in
let host_scenario = fun value =>
  if std.is_string value then value == "HostPause"
  else std.is_record value && (std.record.has_field "HostTrap" value || std.record.has_field "HostUncertain" value) in
# The operator supplies every identity. This is authoring, not evidence admission.
# r[impl molten.spacewasm_core.profile]
let OperatorContract = std.contract.Sequence [{
  profile | ProfileContract,
  trust | Cohort,
  name | P.contract (P.is_one_of ["molten-abi-roundtrip", "molten-abi-hostcall",
    "molten-abi-invalid-pointer", "molten-abi-invalid-output", "molten-abi-memory-limit",
    "molten-abi-stack-limit", "molten-abi-trap", "molten-abi-out-of-fuel"]),
  scenario | Scenario,
  bundle | Relative,
  octet | Relative,
  module | Relative,
  probe | Relative,
  input | Relative,
  chaos_report | Relative,
}, P.contract (fun config => !host_scenario config.scenario || config.name == "molten-abi-hostcall")] in
{
  Profile = ProfileContract,
  Limits = LimitsContract,
  OperatorConfiguration = OperatorContract,
  non_claims = NonClaims,
}
