{
  schema_version = "ucan-nickel-contracts.policy.v1",
  contracts = {
    "component-blob-get" = {
      id = "component-blob-get",
      description = "Read a canonical blob through the inactive component effect host adapter.",
      resource_prefixes = ["molten://blob/"],
      abilities = ["blob/get"],
    },
  },
  backends = {},
}
