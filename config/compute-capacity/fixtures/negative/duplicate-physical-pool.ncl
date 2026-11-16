let Base = import "../positive/agent-a-serving-b.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & {
  offers | force = std.array.map (fun offer =>
    if offer.host_id == "host-b" then offer & {
      pools | force = [
        std.array.first offer.pools,
        std.array.first offer.pools,
        std.array.first (std.array.filter (fun pool => pool.id == "host-b-chip") offer.pools),
      ],
    } else offer
  ) Base.offers,
}) | Contracts.Consumer
