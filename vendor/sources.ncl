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
  file_filter | optional | {
    extensions | Array NonEmpty,
    exact_paths | Array RelativePath,
  },
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
  schema = "molten.vendored-sources.v2",
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
      {
        name = "noble-pure",
        repository = "https://github.com/OnixResearch/noble-lang",
        revision = "f7af9e972aa2968d22978794a667d267cf2745e8",
        license = "AGPL-3.0-or-later (operator exact-revision grant; no upstream license declaration)",
        destination = "vendor/noble-pure",
        purpose = "Opt-in sequential pure I64 net evaluation and independently bounded producer reference evaluator.",
        disclosure = "2026-11-17 operator asserted necessary rights and authorized AGPL-3.0-or-later source-only reuse/redistribution of Onix-owned Rust and Cargo metadata in exactly noble-ngraph, noble-kernel and noble-contracts, then separately granted the indispensable crates/noble-contracts/src/fixtures/result.noble asset (BLAKE3 694a6766ca91c9434329df739070f7b26dda43576dfc3788cb7b7c2c271cb4fe). All other non-Rust fixture programs, third-party code, proofs, integration test suites, other crates and Git history are excluded. Original selected blobs are preserved; no upstream license declaration is invented.",
        selected_paths = [
          "Cargo.toml", "Cargo.lock",
          "crates/noble-ngraph/Cargo.toml", "crates/noble-ngraph/src",
          "crates/noble-kernel/Cargo.toml", "crates/noble-kernel/src",
          "crates/noble-contracts/Cargo.toml", "crates/noble-contracts/src",
        ],
        file_filter = {
          extensions = [".rs"],
          exact_paths = [
            "Cargo.toml", "Cargo.lock",
            "crates/noble-ngraph/Cargo.toml",
            "crates/noble-kernel/Cargo.toml",
            "crates/noble-contracts/Cargo.toml",
            "crates/noble-contracts/src/fixtures/result.noble",
          ],
        },
        renamed_originals = [
          { upstream = "Cargo.toml", destination = "Cargo.toml.orig" },
          { upstream = "crates/noble-ngraph/Cargo.toml", destination = "crates/noble-ngraph/Cargo.toml.orig" },
          { upstream = "crates/noble-kernel/Cargo.toml", destination = "crates/noble-kernel/Cargo.toml.orig" },
          { upstream = "crates/noble-contracts/Cargo.toml", destination = "crates/noble-contracts/Cargo.toml.orig" },
          { upstream = "crates/noble-ngraph/src/net.rs", destination = "crates/noble-ngraph/src/net.rs.orig" },
          { upstream = "crates/noble-ngraph/src/eval.rs", destination = "crates/noble-ngraph/src/eval.rs.orig" },
        ],
        aspen_owned = [
          { path = "crates/noble-ngraph/Cargo.toml", derived_from = "crates/noble-ngraph/Cargo.toml.orig", change = "scripts/normalize-noble-manifests.py resolves package inheritance and unsafe-code lint; no third-party dependencies." },
          { path = "crates/noble-kernel/Cargo.toml", derived_from = "crates/noble-kernel/Cargo.toml.orig", change = "scripts/normalize-noble-manifests.py resolves package inheritance and unsafe-code lint." },
          { path = "crates/noble-contracts/Cargo.toml", derived_from = "crates/noble-contracts/Cargo.toml.orig", change = "scripts/normalize-noble-manifests.py resolves inheritance; omits integration-test-only dev dependency." },
          { path = "crates/noble-ngraph/src/net.rs", derived_from = "crates/noble-ngraph/src/net.rs.orig", change = "Aspen bounded phase/control observation overlay; retains original in-place sequential arena, checked rewrite plans, arithmetic and complete ordered readback." },
          { path = "crates/noble-ngraph/src/eval.rs", derived_from = "crates/noble-ngraph/src/eval.rs.orig", change = "Aspen per-node observation overlay for independently bounded normative source-IR reference evaluator; unchanged arithmetic." },
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
