let Contracts = import "../../contracts.ncl" in
let Revision = "025d9636f0161777710dac37b3c210ca0ad9483f" in
let Source = "rad://z37R1bP1kHcELs89RNbQRaqbCVKxB" in
{
  schema = "molten.release-dependency-profile.v1",
  dependencies = [
    {
      manifest_dependency = "executable-extent-core",
      package_name = "executable-extent-core",
      package_version = "0.1.0",
      source_kind = "git",
      source_coordinate = Source,
      immutable_revision = Revision,
      nix_input = "executable-extent-src",
      transport_policy = "https",
      disposition = "optional-runtime",
    }
  ],
  canonical_valence = {
    source_coordinate = "https://github.com/OnixResearch/valence.git",
    immutable_revision = "5f1c2ba5072c6f9622fa59b1af20502985f569fd",
  },
  archive_receipts = [
    {
      id = "executable-extent",
      source_coordinate = Source,
      source_revision = Revision,
      nix_input = "executable-extent-src",
      archive_path = ".cairn/archive/executable-extent",
      status = "archived",
      evidence_files = [
        {
          relative_path = "evidence/verification.md",
          blake3 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        }
      ],
      non_claims = ["does not prove release eligibility"],
    }
  ],
  distribution = {
    license = "AGPL-3.0-or-later",
    source_coordinate = "https://github.com/example/project.git",
    source_revision = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    notice_artifacts = ["LICENSE"],
    source_export_artifacts = ["Cargo.toml"],
    non_claims = [
      "not legal advice",
      "does not prove compliance in every jurisdiction",
    ],
  },
} | Contracts.Profile
