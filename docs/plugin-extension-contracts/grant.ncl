let Ref = String in
let Attenuation = {
  delegated_scope | String,
  current_delegation_depth | Number,
  max_delegation_depth | Number,
  budget_refs | Array Ref,
  valid_from_turn | Number,
  valid_until_turn | Number,
} in
let PluginCapabilityGrant = {
  plugin_ref | Ref,
  plugin_id | String,
  manifest_ref | Ref,
  extension_contract_ref | Ref,
  hostcall_descriptor_ref | Ref,
  operation | String,
  input_schema_ref | Ref,
  output_schema_ref | Ref,
  resource_refs | Array Ref,
  resource_scope | String,
  effect_manifest_refs | Array Ref,
  effect_receipt_refs | Array Ref,
  policy_refs | Array Ref,
  issuer_ref | Ref,
  proof_refs | Array Ref,
  attenuation | Attenuation,
  revocation_refs | Array Ref,
  revoked | Bool,
  replay_class | String,
} in
PluginCapabilityGrant
