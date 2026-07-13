let Template = import "../../profile-template.ncl" in
Template.make {
  iroh_blobs = {
    backend_hints_are_identity = true,
  },
}
