let Valid = import "../../profiles/logical-dogfood.ncl" in
let Contracts = import "../../contracts.ncl" in
let OneOperation = 1 in
(Valid & {
  limits.max_operations | force = OneOperation,
}) | Contracts.Request
