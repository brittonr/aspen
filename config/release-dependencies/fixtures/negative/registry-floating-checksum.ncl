let Contracts = import "../../contracts.ncl" in
let Revision = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in
{
  schema = "molten.release-dependency-profile.v2",
  dependencies = [{
    manifest_dependency = "pinned-package",
    package_name = "pinned-package",
    package_version = "1.0.0",
    source_kind = "git",
    source_coordinate = "https://github.com/example/pinned.git",
    immutable_revision = Revision,
    nix_input = "pinned-src",
    transport_policy = "https",
    disposition = "runtime",
  }],
  registry_dependencies = [{
    manifest_dependency = "registry-package",
    package_name = "registry-package",
    package_version = "1.0.0",
    source_kind = "crates-io",
    source_coordinate = "registry+https://github.com/rust-lang/crates.io-index",
    checksum = "latest",
    transport_policy = "crates.io",
    plane = "implementation",
    disposition = "optional-runtime",
  }],
  canonical_valence = {
    source_coordinate = "https://github.com/OnixResearch/valence.git",
    immutable_revision = "5f1c2ba5072c6f9622fa59b1af20502985f569fd",
  },
  archive_receipts = [{
    id = "fixture-receipt",
    source_coordinate = "https://github.com/example/pinned.git",
    source_revision = Revision,
    nix_input = "pinned-src",
    archive_path = "cairn/archive/fixture",
    status = "archived",
    evidence_files = [{
      relative_path = "cairn/archive/fixture/tasks.md",
      blake3 = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
    }],
    non_claims = ["fixture receipt grants no authority"],
  }],
  distribution = {
    license = "AGPL-3.0-or-later",
    source_coordinate = "https://github.com/example/project.git",
    source_revision = Revision,
    notice_artifacts = ["LICENSE"],
    source_export_artifacts = ["Cargo.toml"],
    non_claims = ["fixture is not legal advice"],
  },
} | Contracts.Profile
