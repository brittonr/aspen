let Template = import "../../profile-template.ncl" in
Template.make {
  deterministic_simulation = {
    verify_before_exposure = false,
    persist_verified_partial_state = false,
  },
}
