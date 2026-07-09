let Contracts = import "contracts.ncl" in
{
  schema = "molten.dependency-boundary-policy.v1",
  generated_exemptions = ["generated-shard-compatibility"],
  rules = [
    {
      id = "core-no-adapters",
      owning_layer = "core",
      source_globs = ["crates/molten-core/src/**"],
      denied_targets = ["src/cli/**", "src/adapters/**", "redb", "iroh", "wasmtime", "steel-core", "nickel-lang"],
      diagnostic = "pure core code must receive parsed facts and plans rather than importing adapter or live tooling dependencies",
      exemption_class = "generated-shard-compatibility",
    },
    {
      id = "runtime-no-cli-shell",
      owning_layer = "runtime",
      source_globs = ["src/runtime/**", "src/node/parts/runtime/**"],
      denied_targets = ["src/main.rs", "src/cli/**"],
      diagnostic = "runtime modules return planned effects and must not depend on CLI shells",
      exemption_class = "generated-shard-compatibility",
    },
    {
      id = "codec-no-high-domain",
      owning_layer = "codec",
      source_globs = ["src/preserves/**", "src/schema/**"],
      denied_targets = ["src/node/**", "src/job/**", "src/operator/**"],
      diagnostic = "shared codec helpers must not import high-level domains",
      exemption_class = "generated-shard-compatibility",
    },
  ],
} | Contracts.Policy
