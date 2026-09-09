# Negative fixture: profile spec effects must use the checked policy enum.
let Policy = import "../contracts.ncl" in
let Base = import "../default.ncl" in

(Base & {
  schemas = std.array.map (fun schema =>
    if schema.id == "no-spec-delta" then
      schema & { spec_effect = "ambient" }
    else
      schema
  ) Base.schemas,
}) | Policy
