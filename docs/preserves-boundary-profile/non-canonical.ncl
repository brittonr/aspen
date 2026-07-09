let Contracts = import "contracts.ncl" in
let Base = import "valid.ncl" in
Base & {
  measurements = std.array.map (fun measurement =>
    if measurement.artifact_family == "node-control-envelope" then measurement & { canonical_bytes = false } else measurement
  ) Base.measurements,
} | Contracts.Profile
