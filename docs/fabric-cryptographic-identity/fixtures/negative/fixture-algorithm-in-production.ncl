let Template = import "../../profile-template.ncl" in
Template.make {
  algorithm = "blake3-local-fixture-v1",
  entropy_source_kind = "deterministic-fixture",
  backend_class = "deterministic-fixture",
  fixture_only = true,
  capability_rooted = false,
  owner_only_permissions = false,
}
