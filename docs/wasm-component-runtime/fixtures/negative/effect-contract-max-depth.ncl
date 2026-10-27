let Contracts = import "../../effect-contracts.ncl" in
(Contracts.default & {
  max_depth | force = 33,
}) | Contracts.Export
