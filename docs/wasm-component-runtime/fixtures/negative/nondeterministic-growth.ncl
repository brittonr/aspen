let Template = import "../../profile-template.ncl" in
Template.make {
  relaxed_simd = true,
  memory_growth = "dynamic",
}
