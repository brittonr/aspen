let Template = import "../../profile-template.ncl" in
let Contracts = import "../../contracts.ncl" in
Template.make {
  backend_class = "tpm2-sealed",
  purposes = Contracts.purposes,
}
