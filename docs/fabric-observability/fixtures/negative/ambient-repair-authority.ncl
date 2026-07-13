let Template = import "../../profile-template.ncl" in
Template.make {
  read_only_by_default = false,
  separate_mutation_authority_required = false,
}
