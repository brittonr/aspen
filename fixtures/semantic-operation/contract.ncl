let ExactRevision = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.is_match "^[0-9a-f]{40}$" value
) in
let Blake3Hex = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.is_match "^[0-9a-f]{64}$" value
) in
let Identity = {
  domain | std.contract.from_predicate (fun value => value == "effect-operation"),
  algorithm | std.contract.from_predicate (fun value => value == "blake3"),
  hex | Blake3Hex,
} in
let Surfaces = {
  manifest | Identity,
  handler_binding | Identity,
  handle | Identity,
  request | Identity,
  response | Identity,
  effect_log | Identity,
  adapter_import | Identity,
  remote_execution | Identity,
  runtime_receipt | Identity,
  replay_identity | Identity,
  evaluation_cache_key | Identity,
  job | Identity,
  upgrade_check | Identity,
} in
{
  Mapping = {
    schema | std.contract.from_predicate (fun value =>
      value == "molten.semantic-operation-mapping.v1"
    ),
    kamacite_revision | ExactRevision,
    operation | Identity,
    surfaces | Surfaces,
    expected | std.contract.from_predicate (fun value =>
      value == "pass" || value == "deny"
    ),
  },
}
