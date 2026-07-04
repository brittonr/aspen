let Prelude = import "../nickel-contract-prelude.ncl" in

let AllowedReplayClasses = ["idempotent", "deterministic", "effectful"] in

let Ref = Prelude.contracts.Blake3Ref in
let NonEmptyString = Prelude.contracts.NonEmptyString in
let NonNegativeInteger = Prelude.contracts.NonNegativeInteger in
let NonEmptyRefArray = std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array Ref] in
let PluginId = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match "^plugin:[A-Za-z0-9_.:-]+$" value
) in
let Operation = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match "^[a-z][A-Za-z0-9_.:-]+$" value
) in
let ReplayClass = Prelude.contract (Prelude.is_one_of AllowedReplayClasses) in

let attenuation_is_coherent = fun attenuation =>
  std.is_record attenuation
  && attenuation.current_delegation_depth <= attenuation.max_delegation_depth
  && attenuation.valid_from_turn < attenuation.valid_until_turn in
let grant_invariants_hold = fun grant =>
  std.is_record grant
  && (!grant.revoked || Prelude.is_non_empty_array grant.revocation_refs) in

let Attenuation = std.contract.Sequence [{
  delegated_scope | Ref,
  current_delegation_depth | NonNegativeInteger,
  max_delegation_depth | NonNegativeInteger,
  budget_refs | NonEmptyRefArray,
  valid_from_turn | NonNegativeInteger,
  valid_until_turn | NonNegativeInteger,
}, Prelude.contract attenuation_is_coherent] in
let PluginCapabilityGrant = std.contract.Sequence [{
  plugin_ref | Ref,
  plugin_id | PluginId,
  manifest_ref | Ref,
  extension_contract_ref | Ref,
  hostcall_descriptor_ref | Ref,
  operation | Operation,
  input_schema_ref | Ref,
  output_schema_ref | Ref,
  resource_refs | NonEmptyRefArray,
  resource_scope | Ref,
  effect_manifest_refs | NonEmptyRefArray,
  effect_receipt_refs | NonEmptyRefArray,
  policy_refs | NonEmptyRefArray,
  issuer_ref | Ref,
  proof_refs | NonEmptyRefArray,
  attenuation | Attenuation,
  revocation_refs | Array Ref,
  revoked | Bool,
  replay_class | ReplayClass,
}, Prelude.contract grant_invariants_hold] in
PluginCapabilityGrant
