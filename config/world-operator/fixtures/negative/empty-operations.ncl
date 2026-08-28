let Valid = import "../../profiles/logical-dogfood.ncl" in
let Contracts = import "../../contracts.ncl" in
(Valid & {
  operations | force = [],
}) | Contracts.Request
