let Contracts = import "contracts.ncl" in
let Fixture = import "fixture.ncl" in
std.record.update "measurements" (std.array.map (fun measurement =>
  if measurement.artifact_family == "evidence-envelope" then std.record.update "consumer" "ambient-debugger" measurement else measurement
) Fixture.measurements) Fixture
| Contracts.Profile
