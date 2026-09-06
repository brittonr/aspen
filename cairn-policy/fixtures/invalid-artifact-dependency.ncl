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
              artifacts | force =
                std.array.map
                  (fun artifact =>
                    if artifact.id == "tasks" then
                      artifact & { requires | force = ["missing"] }
                    else
                      artifact
                  )
                  schema.artifacts,
            }
          else
            schema
        )
        Base.schemas,
  }
) | Contracts.Policy
