let Valid = import "../../profiles/opaque-replay.ncl" in
let Contracts = import "../../contracts.ncl" in
let UnknownOperationRef = "blake3:501f8eefa33fd4a96742ff83b8997d63278f6ec1c9da790a6ecfe68b8b82de26" in
let OpaqueProfileRef = "blake3:c44f7cbbc1c6e7d7891c4685de9803c07068ad010c6a5322c5299bec1beb32a3" in
let OpaqueWorldRef = "blake3:76874bf8e2d46f80b9c53ab7ee2e9f2dc4f502765cd3f36f09864e61be311362" in
(Valid & {
  operations | force = [{
    operation_id = UnknownOperationRef,
    kind = "shell",
    subject_ref = OpaqueWorldRef,
    profile_ref = OpaqueProfileRef,
    dependencies = [],
  }],
}) | Contracts.Request
