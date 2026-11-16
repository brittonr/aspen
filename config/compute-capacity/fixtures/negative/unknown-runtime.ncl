let Base = import "../positive/agent-a-serving-b.ncl" in
let Contracts = import "../../contracts.ncl" in
let Serving = std.array.first Base.steps in
(Base & {
  offers | force = std.array.map (fun offer =>
    if offer.host_id == "host-b" then offer & {
      compatibility | force = offer.compatibility & { runtime | force = null },
    } else offer
  ) Base.offers,
  steps | force = [Serving],
}) | Contracts.Consumer
