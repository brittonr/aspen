let Contracts = import "../../contracts.ncl" in
let RevisionA = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in
let RevisionB = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" in
{
  schema = "molten.release-dependency-profile.v1",
  dependencies = [
    {
      manifest_dependency = "first-package",
      package_name = "first-package",
      package_version = "1.0.0",
      source_kind = "git",
      source_coordinate = "ssh://git@github.com/example/first.git",
      immutable_revision = RevisionA,
      nix_input = "shared-src",
      transport_policy = "ssh-pinned-nix-archive",
      disposition = "runtime",
    },
    {
      manifest_dependency = "second-package",
      package_name = "second-package",
      package_version = "1.0.0",
      source_kind = "git",
      source_coordinate = "ssh://git@github.com/example/second.git",
      immutable_revision = RevisionB,
      nix_input = "shared-src",
      transport_policy = "ssh-pinned-nix-archive",
      disposition = "runtime",
    },
  ],
  canonical_valence = {
    source_coordinate = "https://github.com/OnixResearch/valence.git",
    immutable_revision = RevisionA,
  },
  archive_receipts = [{
    id = "fixture-receipt",
    source_coordinate = "https://github.com/example/first.git",
    source_revision = RevisionA,
    nix_input = "shared-src",
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
    source_revision = RevisionA,
    notice_artifacts = ["LICENSE"],
    source_export_artifacts = ["Cargo.toml"],
    non_claims = ["fixture is not legal advice"],
  },
} | Contracts.Profile
