let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { oracle_ref | force = "blake3:14BAC5BF" }) | Contracts.Profile
