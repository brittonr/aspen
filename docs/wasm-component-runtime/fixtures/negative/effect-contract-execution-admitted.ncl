let Contracts = import "../../effect-contracts.ncl" in
(Contracts.default & {
  execution_admitted | force = true,
}) | Contracts.Export
