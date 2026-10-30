let Base = import "../profile.ncl" in
let C = import "../../contracts.ncl" in
(Base & { scope | force = "cluster" }) | C.Profile
