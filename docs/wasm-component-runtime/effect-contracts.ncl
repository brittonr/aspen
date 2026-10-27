# Immutable product-owned wire/linker contract, not execution admission.
let Default = {
  schema_id = "molten.wasm-component-effects.v1",
  schema_version = 1,
  profile_id = "molten.wasm.component.effects.v1",
  execution_admitted = false,
  wit = {
    package = "molten:component-effects@1.0.0",
    world = "actor-effects",
    source_ref = "blake3:9478f7ddd9380bcbe898f2101d615674067e0f167ddc6f0fc345553751b5eaa2",
  },
  allowed_imports = ["effect-request"],
  allowed_exports = ["invoke"],
  allowed_wasi_interfaces = [],
  closed_operations = [{
    effect_id = "blob.get",
    operation = "get",
    capability = "hostcall:blob-get",
  }],
  request_schema = "molten.component-effect-request.v1",
  response_schema = "molten.component-effect-response.v1",
  max_request_bytes = 65536,
  max_response_bytes = 65536,
  max_depth = 32,
  max_nodes = 4096,
  required_current_inputs = [
    "policy", "basalt-ucan", "generation", "incarnation",
    "resources", "recorded-effect", "request-sequence",
  ],
  non_claims = [
    "carrier-is-not-authority",
    "decoding-is-not-admission",
    "recording-is-not-effect-success",
    "contract-is-not-consumer-installation",
  ],
} in
{
  default = Default,
  Export = std.contract.from_predicate (fun candidate => candidate == Default),
}
