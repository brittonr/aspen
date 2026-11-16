# A syntactically stable identifier is not a canonical provider-profile reference.
let Base = import "../positive/agent-a-serving-b.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & {
  offers | force = std.array.map (fun offer =>
    if offer.host_id == "host-b" then offer & { provider_profile_ref | force = "synthetic-provider-host-b" }
    else offer
  ) Base.offers,
}) | Contracts.Consumer
