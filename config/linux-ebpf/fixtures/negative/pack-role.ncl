let Base = import "../profile.ncl" in
let C = import "../../contracts.ncl" in
(Base & { binding.pack | force = Base.profile_ref }) | C.Profile
