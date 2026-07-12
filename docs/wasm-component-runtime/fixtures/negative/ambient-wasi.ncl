let Template = import "../../profile-template.ncl" in
Template.make {
  allowed_imports = ["wasi:filesystem/types@0.2.6"],
  allowed_wasi_interfaces = ["wasi:filesystem/types@0.2.6"],
}
