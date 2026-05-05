# Aspen feature-bundle policy contracts.
#
# Nickel validates human-facing product/profile bundles. Cargo/Rust keep final
# feature resolution and compilation semantics.

let contains = fun values value =>
  std.array.fold_left (fun found item => found || item == value) false values in

let enum_string = fun field allowed =>
  std.contract.from_validator (fun value =>
    if std.is_string value && contains allowed value then
      'Ok
    else
      'Error { message = "%{field} must be one of %{std.serialize 'Json allowed}" }
  ) in

let non_empty = fun field =>
  std.contract.from_validator (fun value =>
    if std.is_string value && std.string.length value > 0 then
      'Ok
    else
      'Error { message = "%{field} must be a non-empty string" }
  ) in

let ProfileName = enum_string "profile.name" [
  "minimal",
  "dogfood",
  "ci-worker",
  "forge",
  "snix",
  "full",
] in

let FeatureName = enum_string "feature" [
  "sql",
  "forge",
  "git-bridge",
  "ci",
  "ci-basic",
  "secrets",
  "automerge",
  "global-discovery",
  "plugins",
  "shell-worker",
  "blob",
  "docs",
  "hooks",
  "jobs",
  "federation",
  "proxy",
  "snix",
  "snix-http",
  "snix-daemon",
  "snix-eval",
  "snix-build",
  "trust",
  "nix-cli-fallback",
] in

let ProductProfile = {
  name | ProfileName,
  description | non_empty "profile.description",
  features | Array FeatureName | default = [],
  requires | Array FeatureName | default = [],
  forbids | Array FeatureName | default = [],
  operator_notes | Array String | default = [],
} in

let BundlePolicy = {
  schema_version | String | default = "1",
  profiles | Array ProductProfile,
} in

let ProfileNameContract = ProfileName in
let FeatureNameContract = FeatureName in
let ProductProfileContract = ProductProfile in
let BundlePolicyContract = BundlePolicy in
{
  ProfileName = ProfileNameContract,
  FeatureName = FeatureNameContract,
  ProductProfile = ProductProfileContract,
  BundlePolicy = BundlePolicyContract,

  supported | BundlePolicy = {
    profiles = [
      {
        name = "minimal",
        description = "Cluster bootstrap without application feature bundles.",
        features = [],
      },
      {
        name = "dogfood",
        description = "Self-hosting profile for Forge, CI, jobs, blobs, snix and trust references.",
        features = ["forge", "ci", "jobs", "blob", "docs", "snix", "snix-build", "trust"],
      },
      {
        name = "ci-worker",
        description = "Worker profile for CI job execution and artifact publication.",
        features = ["ci", "ci-basic", "jobs", "shell-worker", "blob"],
      },
      {
        name = "forge",
        description = "Forge hosting profile with Git bridge and blob-backed repositories.",
        features = ["forge", "git-bridge", "blob", "docs"],
      },
      {
        name = "snix",
        description = "Nix cache/eval/build integration profile without owning Nix build semantics.",
        features = ["snix", "snix-http", "snix-daemon", "snix-eval", "snix-build", "blob"],
      },
      {
        name = "full",
        description = "Operator-selected all-capability profile.",
        features = ["sql", "forge", "git-bridge", "ci", "ci-basic", "secrets", "automerge", "global-discovery", "plugins", "shell-worker", "blob", "docs", "hooks", "jobs", "federation", "proxy", "snix", "snix-http", "snix-daemon", "snix-eval", "snix-build", "trust", "nix-cli-fallback"],
      },
    ],
  },
}
