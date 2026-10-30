{
  schema_version = "ucan-nickel-contracts.policy.v1",
  backends = {},
  contracts = {
    "collaboration-scope" = {
      id = "collaboration-scope",
      description = "Exact collaboration resource and scope operations; no inherited sharing.",
      abilities = ["resource/read", "resource/use", "resource/share", "resource/bind", "resource/move", "scope/create", "scope/membership"],
      resource_prefixes = ["molten://collaboration/"],
    },
  },
}
