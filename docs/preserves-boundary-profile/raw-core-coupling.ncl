let Contracts = import "contracts.ncl" in
let Base = import "valid.ncl" in
Base & {
  measurements = std.array.map (fun measurement =>
    if measurement.artifact_family == "receipt" then measurement & { raw_preserves_core_coupling = true } else measurement
  ) Base.measurements,
} | Contracts.Profile
