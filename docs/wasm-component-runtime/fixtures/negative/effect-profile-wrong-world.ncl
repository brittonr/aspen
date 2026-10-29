let Template = import "../../profile-template.ncl" in
let Effect = (import "../../effect-contracts.ncl").default in
Template.make {
  profile_id = Effect.profile_id,
  allowed_imports = Effect.allowed_imports,
}
