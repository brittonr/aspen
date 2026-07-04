let PluginExtensionContract = import "contract.ncl" in
{
  extension_id = "plugin-extension:storage",
  version = "1.0.0",
  compatible_host_abi = "molten.plugin.host-abi.v1",
  lifecycle_callbacks = ["start", "health"],
  hostcalls = [{
    operation = "storage.read",
    descriptor_ref = "blake3:1111111111111111111111111111111111111111111111111111111111111111",
    input_schema_ref = "blake3:2222222222222222222222222222222222222222222222222222222222222222",
    output_schema_ref = "blake3:3333333333333333333333333333333333333333333333333333333333333333",
    authority_refs = ["blake3:4444444444444444444444444444444444444444444444444444444444444444"],
    resource_refs = ["blake3:5555555555555555555555555555555555555555555555555555555555555555"],
    effect_manifest_refs = ["blake3:6666666666666666666666666666666666666666666666666666666666666666"],
    replay_class = "idempotent",
    error_class_refs = ["blake3:7777777777777777777777777777777777777777777777777777777777777777"],
  }],
  conformance = {
    positive_suite_ref = "blake3:8888888888888888888888888888888888888888888888888888888888888888",
    negative_suite_ref = "blake3:9999999999999999999999999999999999999999999999999999999999999999",
    property_suite_ref = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  },
  policy_refs = ["blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"],
  supply_chain_refs = ["blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"],
  profile = "production",
} | PluginExtensionContract
