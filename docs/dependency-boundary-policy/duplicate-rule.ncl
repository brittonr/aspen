let Contracts = import "contracts.ncl" in
{
  schema = "molten.dependency-boundary-policy.v1",
  rules = [
    {
      id = "core-no-adapters",
      owning_layer = "core",
      source_globs = ["crates/molten-core/src/**"],
      denied_targets = ["src/adapters/**"],
      diagnostic = "core cannot import adapters",
      exemption_class = "generated-shard-compatibility",
    },
    {
      id = "core-no-adapters",
      owning_layer = "runtime",
      source_globs = ["src/runtime/**"],
      denied_targets = ["src/main.rs"],
      diagnostic = "runtime cannot import CLI shell",
      exemption_class = "generated-shard-compatibility",
    },
  ],
} | Contracts.Policy
