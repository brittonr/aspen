# Test harness suite metadata schema.
#
# Authoritative schema for per-suite Nickel manifests. Exporters validate each
# manifest against `SuiteManifest`, then normalize the results into committed
# machine-consumable inventory data.

let contains = fun values value =>
  std.array.fold_left (fun found item => found || item == value) false values in

let enum_string = fun field allowed =>
  std.contract.from_validator (fun value =>
    if std.is_string value && contains allowed value then
      'Ok
    else
      'Error {
        message = "%{field} must be one of %{std.serialize 'Json allowed}",
      }
  ) in

let layer_contract = enum_string "layer" [
  "rust-integration",
  "patchbay",
  "vm",
] in

let runtime_class_contract = enum_string "runtime_class" [
  "real-network",
  "linux-namespaces",
  "nixos-vm",
] in

let prerequisite_contract = enum_string "prerequisite" [
  "network-access",
  "linux-userns",
  "nested-kvm",
  "nftables",
  "traffic-control",
  "nix-command",
] in

let target_kind_contract = enum_string "target.kind" [
  "cargo-nextest",
  "nix-build",
] in

let run_ignored_contract = enum_string "target.run_ignored" [
  "default",
  "all",
  "ignored-only",
] in

let timeout_class_contract = enum_string "timeout_class" [
  "quick",
  "standard",
  "extended",
  "soak",
] in

let fault_dimension_contract = enum_string "fault.dimension" [
  "latency",
  "loss",
  "partition",
  "nat",
  "jitter",
] in

let ArtifactExpectation = {
  path | String,
  required | Bool | default = true,
  kind | String | default = "file",
} in

let FaultDimension = {
  dimension | fault_dimension_contract,
  bound | String,
  expected_convergence | String | optional,
} in

{
  SuiteManifest = {
    id | String,
    display_name | String | optional,
    description | String | optional,
    layer | layer_contract,
    owner | String,
    runtime_class | runtime_class_contract,
    prerequisites | Array prerequisite_contract | default = [],
    tags | Array String | default = [],
    capabilities | Array String | default = [],
    isolation_assumptions | Array String | default = [],
    timeout_class | timeout_class_contract | default = "standard",
    expected_artifacts | Array ArtifactExpectation | default = [],
    faults | Array FaultDimension | default = [],
    convergence_assertions | Array String | default = [],
    target | {
      kind | target_kind_contract,
      package | String | optional,
      test | String | optional,
      profile | String | optional,
      features | Array String | default = [],
      run_ignored | run_ignored_contract | optional,
      flake_attr | String | optional,
      check_attr | String | optional,
      nix_file | String | optional,
      package_presets | {_ : String} | default = {},
      register_flake_check | Bool | default = false,
    },
  },
}
