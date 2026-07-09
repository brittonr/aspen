let Contracts = import "contracts.ncl" in
let Base = import "valid.ncl" in
Base & {
  measurements = std.array.map (fun measurement =>
    if measurement.artifact_family == "workflow-bundle" then measurement & { blake3_ref = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" } else measurement
  ) Base.measurements,
} | Contracts.Profile
