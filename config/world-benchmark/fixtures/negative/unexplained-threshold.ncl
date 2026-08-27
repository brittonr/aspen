let Valid = import "../../profiles/logical-synthetic.ncl" in
let Contracts = import "../../contracts.ncl" in
(Valid & {
  profile.thresholds | force = [{
    name = "",
    metric = "physical-bytes-written",
    maximum = 1,
    operation = "root-branch",
  }],
}) | Contracts.Input
