let Template = import "../../profile-template.ncl" in
Template.make {
  allow_unbounded_queue = true,
  terminal_failures_explicit = false,
}
