let Template = import "../../profile-template.ncl" in
Template.make {
  iroh_blobs = {
    live_network_required = false,
  },
}
