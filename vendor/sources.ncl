# Exact upstream source selections that ship with Aspen, and repository fixtures
# bound to exact upstream files. This file is reviewed policy: which files are
# selected, why, and under which terms. `vendor/sources.lock.json` is generated
# by `scripts/vendored-sources.py import` and verified by
# `scripts/vendored-sources.py check`.
let Revision =
  std.contract.from_predicate (fun value => std.is_string value && std.string.is_match "^[0-9a-f]{40}$" value)
in
let RelativePath =
  std.contract.from_predicate (fun value =>
    std.is_string value
    && value != ""
    && !(std.string.is_match "^/" value)
    && !(std.string.is_match "(^|/)\\.\\.?(/|$)" value)
  )
in
let NonEmpty = std.contract.from_predicate (fun value => std.is_string value && value != "") in
let Source = {
  name | NonEmpty,
  repository | NonEmpty,
  revision | Revision,
  license | NonEmpty,
  destination | RelativePath,
  purpose | NonEmpty,
  disclosure | NonEmpty,
  selected_paths | Array RelativePath,
  renamed_originals | Array { upstream | RelativePath, destination | RelativePath },
  aspen_owned | Array { path | RelativePath, derived_from | RelativePath, change | NonEmpty },
}
in
let FixtureBinding = {
  repository | NonEmpty,
  revision | Revision,
  upstream_path | RelativePath,
  local_path | RelativePath,
  purpose | NonEmpty,
}
in
{
  schema = "molten.vendored-sources.v1",
  notices | RelativePath = "THIRD_PARTY_LICENSES.md",
  sources
    | Array Source
    = [
      {
        name = "vm-cohort",
        repository = "rad://z2QJLUqyAZnnHPiZQ1BFjLsX9ush3",
        revision = "31f1696ba9391bfda8577a58af84f72361d5573e",
        license = "MIT OR Apache-2.0",
        destination = "vendor/vm-cohort",
        purpose = "Optional `world-snapshot-vm-cohort` dependency `vm-cohort-core`, the focused world-snapshot Octet workspace, and the VM Cohort source-ownership check.",
        disclosure = "Owner-approved source-only publication of exactly these files at this revision (2026-11-15). Git history and all other repository material stay unpublished.",
        selected_paths = [
          "Cargo.toml",
          "Cargo.lock",
          "LICENSE",
          "LICENSE-APACHE",
          "LICENSE-MIT",
          "NOTICE",
          "README.md",
          ".cairn/archive/1970-01-01-build-vm-cohort/tasks.md",
          "crates/vm-cohort-core",
        ],
        renamed_originals = [
          {
            upstream = "crates/vm-cohort-core/Cargo.toml",
            destination = "crates/vm-cohort-core/Cargo.toml.orig",
          },
        ],
        aspen_owned = [
          {
            path = "crates/vm-cohort-core/Cargo.toml",
            derived_from = "crates/vm-cohort-core/Cargo.toml.orig",
            change = "Workspace-inherited package fields, lints and dependency requirements resolved from the selected root Cargo.toml, as `cargo vendor` does, so Cargo can build the crate outside its upstream workspace. No Rust source changes.",
          },
        ],
      },
    ],
  fixture_bindings
    | Array FixtureBinding
    = [
      {
        repository = "rad://z3DJe8tEdQuXpzTkfqCYQq6ZUqqkb",
        revision = "2c636b1b25353a1b0befa5af48dc68615cd686dd",
        upstream_path = "schemas/machine-contracts/fixtures/executable-extent-bundle.valid.json",
        local_path = "tests/fixtures/executable-extent/executable-extent-bundle.valid.json",
        purpose = "Mantle executable-extent producer bundle fixture consumed by the executable-extent consumer contract.",
      },
      {
        repository = "rad://z3DJe8tEdQuXpzTkfqCYQq6ZUqqkb",
        revision = "2c636b1b25353a1b0befa5af48dc68615cd686dd",
        upstream_path = "schemas/machine-contracts/fixtures/executable-extent-producer-receipt.valid.json",
        local_path = "tests/fixtures/executable-extent/executable-extent-producer-receipt.valid.json",
        purpose = "Mantle detached producer-receipt fixture consumed by the executable-extent consumer contract.",
      },
    ],
}
