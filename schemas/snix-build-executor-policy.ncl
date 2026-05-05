# Aspen snix/build executor policy contracts.
#
# Nickel validates operator policy references and safety bounds only. Snix build,
# cache, store, sandbox, and fallback execution behavior remains Rust-owned.

let contains = fun values value =>
  std.array.fold_left (fun found item => found || item == value) false values in

let enum_string = fun field allowed =>
  std.contract.from_validator (fun value =>
    if std.is_string value && contains allowed value then
      'Ok
    else
      'Error { message = "%{field} must be one of %{std.serialize 'Json allowed}" }
  ) in

let positive_int = fun field =>
  std.contract.from_validator (fun value =>
    if std.is_number value && value > 0 && value % 1 == 0 then
      'Ok
    else
      'Error { message = "%{field} must be a positive integer" }
  ) in

let non_empty = fun field =>
  std.contract.from_validator (fun value =>
    if std.is_string value && std.string.length value > 0 then
      'Ok
    else
      'Error { message = "%{field} must be a non-empty string" }
  ) in

let SandboxBackend = enum_string "sandbox.backend" [
  "bubblewrap",
  "oci",
  "hyperlight",
  "disabled",
] in

let CacheMode = enum_string "cache.mode" [
  "read-only",
  "write-through",
  "offline",
] in

let FallbackMode = enum_string "fallback.mode" [
  "disabled",
  "explicit-only",
  "last-resort",
] in

let SandboxPolicy = {
  backend | SandboxBackend | default = "bubblewrap",
  network | Bool | default = false,
  max_seconds | positive_int "sandbox.max_seconds" | default = 3600,
  max_output_bytes | positive_int "sandbox.max_output_bytes" | default = 1073741824,
} in

let UpstreamCache = {
  name | non_empty "upstream_cache.name",
  url | non_empty "upstream_cache.url",
  public_key_ref | String | optional,
  mode | CacheMode | default = "read-only",
} in

let ExecutorPolicy = {
  schema_version | String | default = "1",
  sandbox | SandboxPolicy | default = {},
  upstream_caches | Array UpstreamCache | default = [],
  fallback | FallbackMode | default = "disabled",
  allow_nix_cli_fallback | Bool | default = false,
  trusted_substituter_refs | Array String | default = [],
  denied_path_prefixes | Array String | default = ["/etc", "/root", "/run/secrets"],
} in

let SandboxBackendContract = SandboxBackend in
let CacheModeContract = CacheMode in
let FallbackModeContract = FallbackMode in
let SandboxPolicyContract = SandboxPolicy in
let UpstreamCacheContract = UpstreamCache in
let ExecutorPolicyContract = ExecutorPolicy in
{
  SandboxBackend = SandboxBackendContract,
  CacheMode = CacheModeContract,
  FallbackMode = FallbackModeContract,
  SandboxPolicy = SandboxPolicyContract,
  UpstreamCache = UpstreamCacheContract,
  ExecutorPolicy = ExecutorPolicyContract,
}
