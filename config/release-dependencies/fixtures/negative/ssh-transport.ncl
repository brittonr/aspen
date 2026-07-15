let Contracts = import "../../contracts.ncl" in
{
  schema = "molten.release-dependency-profile.v1",
  dependencies = [{
    manifest_dependency = "ssh-only",
    package_name = "ssh-only",
    package_version = "1.0.0",
    source_kind = "git",
    source_coordinate = "ssh://git@github.com/example/ssh-only.git",
    immutable_revision = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    nix_input = "ssh-only-src",
    transport_policy = "https",
    disposition = "runtime",
  }],
  canonical_valence = {
    source_coordinate = "https://github.com/OnixResearch/valence.git",
    immutable_revision = "5f1c2ba5072c6f9622fa59b1af20502985f569fd",
  },
  archive_receipts = [],
  distribution = {
    license = "AGPL-3.0-or-later",
    source_coordinate = "https://github.com/example/ssh-only.git",
    source_revision = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    notice_artifacts = ["LICENSE"],
    source_export_artifacts = ["Cargo.toml"],
    non_claims = ["not legal advice", "does not prove compliance in every jurisdiction"],
  },
} | Contracts.Profile
