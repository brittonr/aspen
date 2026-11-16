# A healthy authored offer with no qualification evidence is not positive.
let Base = import "../positive/agent-a-serving-b.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & {
  offers | force = std.array.map (fun offer =>
    if offer.host_id == "host-b" then offer & {
      compatibility | force = offer.compatibility & { qualification_refs | force = [] },
    } else offer
  ) Base.offers,
}) | Contracts.Consumer
