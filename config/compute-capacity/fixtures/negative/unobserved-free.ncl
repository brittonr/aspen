let Base = import "../positive/agent-a-serving-b.ncl" in
let Contracts = import "../../contracts.ncl" in
let Serving = std.array.first Base.steps in
(Base & {
  offers | force = std.array.map (fun offer =>
    if offer.host_id == "host-b" then offer & {
      pools | force = std.array.map (fun pool =>
        if pool.id == "host-b-shared-memory" then pool & {
          observed_used | force = null,
          observed_free | force = null,
        } else pool
      ) offer.pools,
    } else offer
  ) Base.offers,
  steps | force = [Serving],
}) | Contracts.Consumer
