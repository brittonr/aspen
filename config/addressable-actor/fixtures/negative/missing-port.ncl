let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
let InvalidPorts = std.record.remove "authority" Base.port_profiles in
(Base & { port_profiles | force = InvalidPorts }) | Contracts.Profile
