let Contracts = import "contracts.ncl" in
let Fixture = import "fixture.ncl" in
std.record.update "measurements" (std.array.map (fun measurement =>
  if measurement.artifact_family == "receipt" then std.record.update "blake3_ref" "blake3:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" measurement else measurement
) Fixture.measurements) Fixture
| Contracts.Profile
