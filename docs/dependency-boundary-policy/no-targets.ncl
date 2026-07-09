let Contracts = import "contracts.ncl" in
{
  schema = "molten.dependency-boundary-policy.v1",
  rules = [
    {
      id = "core-no-adapters",
      owning_layer = "core",
      source_globs = ["crates/molten-core/src/**"],
      denied_targets = [],
      allowed_targets = [],
      diagnostic = "core cannot import adapters",
      exemption_class = "generated-shard-compatibility",
    },
  ],
} | Contracts.Policy
