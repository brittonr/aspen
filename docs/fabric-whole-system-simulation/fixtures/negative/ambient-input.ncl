let Template = import "../../profile-template.ncl" in
Template.make {
  ambient_inputs = ["std.env.HOME"],
}
