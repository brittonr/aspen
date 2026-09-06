let Contracts = import "../contracts.ncl" in
let Base = import "../default.ncl" in
(
  Base
  & {
    schemas | force =
      std.array.map
        (fun schema =>
          if schema.id == "spec-driven" then
            schema
            & {
              artifacts | force = schema.artifacts @ [std.array.first schema.artifacts],
            }
          else
            schema
        )
        Base.schemas,
  }
) | Contracts.Policy
