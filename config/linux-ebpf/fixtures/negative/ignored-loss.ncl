let Base = import "../profile.ncl" in
let C = import "../../contracts.ncl" in
(Base & { loss_policy | force = "ignore-loss" }) | C.Profile
