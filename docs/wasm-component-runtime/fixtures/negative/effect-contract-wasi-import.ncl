let Contracts = import "../../effect-contracts.ncl" in
(Contracts.default & {
  allowed_imports | force = ["effect-request", "wasi:filesystem/types@0.2.6"],
  allowed_wasi_interfaces | force = ["wasi:filesystem/types@0.2.6"],
}) | Contracts.Export
