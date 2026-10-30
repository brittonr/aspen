let Base = import "../profile.ncl" in
let C = import "../../contracts.ncl" in
(Base & { max_records | force = 0 }) | C.Profile
