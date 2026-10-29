let Template = import "profile-template.ncl" in
let Effect = (import "effect-contracts.ncl").default in
Template.make {
  profile_id = Effect.profile_id,
  wit_package = Effect.wit.package,
  wit_world = Effect.wit.world,
  wit_source_ref = Effect.wit.source_ref,
  allowed_imports = Effect.allowed_imports,
}
