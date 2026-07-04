let Ref = String in
let HostcallDescriptor = {
  operation | String,
  descriptor_ref | Ref,
  input_schema_ref | Ref,
  output_schema_ref | Ref,
  authority_refs | Array Ref,
  resource_refs | Array Ref,
  effect_manifest_refs | Array Ref,
  replay_class | String,
  error_class_refs | Array Ref,
} in
let Conformance = {
  positive_suite_ref | Ref,
  negative_suite_ref | Ref,
  property_suite_ref | Ref,
} in
let PluginExtensionContract = {
  extension_id | String,
  version | String,
  compatible_host_abi | String,
  lifecycle_callbacks | Array String,
  hostcalls | Array HostcallDescriptor,
  conformance | Conformance,
  policy_refs | Array Ref,
  supply_chain_refs | Array Ref,
  profile | String,
} in
PluginExtensionContract
