let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { leaf_domain | force = "unreviewed:v1" }) | Contracts.Profile
