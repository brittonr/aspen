let Valid = import "../../profiles/logical-synthetic.ncl" in
let Contracts = import "../../contracts.ncl" in
(Valid & {
  dataset.preexisting_objects | force = 1,
}) | Contracts.Input
