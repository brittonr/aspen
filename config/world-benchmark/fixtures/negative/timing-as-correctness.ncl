let Valid = import "../../profiles/logical-synthetic.ncl" in
let Contracts = import "../../contracts.ncl" in
(Valid & {
  profile.thresholds | force = [{
    name = "duration-as-correctness",
    metric = "duration-nanoseconds",
    maximum = 1,
    operation = "all",
  }],
}) | Contracts.Input
