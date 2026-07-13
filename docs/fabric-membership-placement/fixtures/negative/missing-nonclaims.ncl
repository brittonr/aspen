let Template = import "../../profile-template.ncl" in
Template.make {
  non_claims = ["does-not-prove-consensus"],
}
