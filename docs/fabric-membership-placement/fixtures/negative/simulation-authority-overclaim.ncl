let Template = import "../../profile-template.ncl" in
Template.make {
  provider_kind = "deterministic-simulation",
  authority_strength = "consistency-ordered",
}
