let Template = import "../../profile-template.ncl" in
Template.make {
  capability_local = {
    bounds = { max_total_bytes = 0 },
  },
}
