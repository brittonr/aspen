let Base = import "../profile.ncl" in
let C = import "../../contracts.ncl" in
(Base & { mappings | force = [{ signal = "command-line", descriptor_ref = Base.profile_ref,
  aggregation = "sum", labels = [] }] }) | C.Profile
