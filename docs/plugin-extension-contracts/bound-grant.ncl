let Prelude = import "../nickel-contract-prelude.ncl" in
let PluginCapabilityGrant = import "grant.ncl" in

let descriptor_matches_grant = fun descriptor grant =>
  descriptor.operation == grant.operation
  && descriptor.descriptor_ref == grant.hostcall_descriptor_ref
  && descriptor.input_schema_ref == grant.input_schema_ref
  && descriptor.output_schema_ref == grant.output_schema_ref
  && descriptor.replay_class == grant.replay_class
  && Prelude.has_all descriptor.resource_refs grant.resource_refs
  && std.array.elem grant.resource_scope descriptor.resource_refs
  && Prelude.has_all descriptor.effect_manifest_refs grant.effect_manifest_refs in

let bound_grant_holds = fun contract grant =>
  std.is_record contract
  && std.is_record grant
  && contract.profile == "production"
  && std.array.any (fun descriptor => descriptor_matches_grant descriptor grant) contract.hostcalls in

let BoundGrantForContract = fun contract => std.contract.Sequence [
  PluginCapabilityGrant,
  Prelude.contract (bound_grant_holds contract),
] in

{
  ForContract = BoundGrantForContract,
  bind = fun contract grant => grant | BoundGrantForContract contract,
}
