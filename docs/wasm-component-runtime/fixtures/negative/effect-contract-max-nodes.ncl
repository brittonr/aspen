let Contracts = import "../../effect-contracts.ncl" in
(Contracts.default & {
  max_nodes | force = 4097,
}) | Contracts.Export
