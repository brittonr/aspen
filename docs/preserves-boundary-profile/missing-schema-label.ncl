let Contracts = import "contracts.ncl" in
let Fixture = import "fixture.ncl" in
std.record.update "measurements" (std.array.map (fun measurement =>
  if measurement.artifact_family == "ticket" then std.record.remove "schema_label" measurement else measurement
) Fixture.measurements) Fixture
| Contracts.Profile
