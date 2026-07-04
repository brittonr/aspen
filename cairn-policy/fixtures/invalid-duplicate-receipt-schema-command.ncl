let Contracts = import "../contracts.ncl" in
let Base = import "../default.ncl" in
(Base & {
  receipt_schemas | force = Base.receipt_schemas @ [std.array.at 0 Base.receipt_schemas],
}) | Contracts.Policy
