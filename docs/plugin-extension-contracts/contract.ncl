let Prelude = import "../nickel-contract-prelude.ncl" in

let HostAbiSchema = "molten.plugin.host-abi.v1" in
let AllowedProfiles = ["development", "test", "production"] in
let AllowedLifecycleCallbacks = ["start", "stop", "health", "configure"] in
let AllowedReplayClasses = ["idempotent", "deterministic", "effectful"] in

let Ref = Prelude.contracts.Blake3Ref in
let NonEmptyString = Prelude.contracts.NonEmptyString in
let NonEmptyRefArray = std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array Ref] in
let ExtensionId = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match "^plugin-extension:[A-Za-z0-9_.:-]+$" value
) in
let Operation = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match "^[a-z][A-Za-z0-9_.:-]+$" value
) in
let Version = Prelude.contracts.Semver in
let HostAbi = Prelude.contract (Prelude.is_exact_string HostAbiSchema) in
let LifecycleCallback = Prelude.contract (Prelude.is_one_of AllowedLifecycleCallbacks) in
let ReplayClass = Prelude.contract (Prelude.is_one_of AllowedReplayClasses) in
let Profile = Prelude.contract (Prelude.is_one_of AllowedProfiles) in

let descriptor_identity = fun descriptor => descriptor.operation ++ ":" ++ descriptor.descriptor_ref in
let descriptors_are_unique = fun hostcalls =>
  Prelude.distinct_strings (std.array.map descriptor_identity hostcalls) in
let extension_invariants_hold = fun contract =>
  std.is_record contract
  && descriptors_are_unique contract.hostcalls in
let ExtensionInvariants = Prelude.contract extension_invariants_hold in

let HostcallDescriptor = {
  operation | Operation,
  descriptor_ref | Ref,
  input_schema_ref | Ref,
  output_schema_ref | Ref,
  authority_refs | NonEmptyRefArray,
  resource_refs | NonEmptyRefArray,
  effect_manifest_refs | NonEmptyRefArray,
  replay_class | ReplayClass,
  error_class_refs | NonEmptyRefArray,
} in
let Conformance = {
  positive_suite_ref | Ref,
  negative_suite_ref | Ref,
  property_suite_ref | Ref,
} in
let PluginExtensionContract = std.contract.Sequence [{
  extension_id | ExtensionId,
  version | Version,
  compatible_host_abi | HostAbi,
  lifecycle_callbacks | std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array LifecycleCallback],
  hostcalls | std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array HostcallDescriptor],
  conformance | Conformance,
  policy_refs | NonEmptyRefArray,
  supply_chain_refs | NonEmptyRefArray,
  profile | Profile,
}, ExtensionInvariants] in
PluginExtensionContract
