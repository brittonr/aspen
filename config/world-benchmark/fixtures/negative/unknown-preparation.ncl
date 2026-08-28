let Valid = import "../../profiles/logical-synthetic.ncl" in
let Contracts = import "../../contracts.ncl" in
(Valid & {
  profile.preparation | force = "unknown",
  dataset.preparation | force = "unknown",
}) | Contracts.Input
