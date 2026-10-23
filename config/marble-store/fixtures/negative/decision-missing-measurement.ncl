let Base = import "../../decision.ncl" in
let Contracts = import "../../contracts.ncl" in
(
  Base
  & {
    measurements
      | force
      = std.array.filter (fun item => item.metric != "recovery") Base.measurements,
  }
)
  | Contracts.Decision
