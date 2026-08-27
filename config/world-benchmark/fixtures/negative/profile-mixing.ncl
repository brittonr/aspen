let Valid = import "../../profiles/logical-synthetic.ncl" in
let Contracts = import "../../contracts.ncl" in
(Valid & {
  profile.operations | force = ["snapshot-share"],
}) | Contracts.Input
