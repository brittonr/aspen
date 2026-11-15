let Template = import "../../profile-template.ncl" in
# Shape fixture only: these template refs are not provisioned backend admission.
Template.make { backend_class = "tpm2-sealed" }
