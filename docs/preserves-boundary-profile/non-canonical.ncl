# r[impl molten.runtime_spine.preserves_boundary_profile.fixtures.negative]
let Contracts = import "contracts.ncl" in
let Fixture = import "fixture.ncl" in
std.record.update "measurements" (std.array.map (fun measurement =>
  if measurement.artifact_family == "node-control-envelope" then std.record.update "canonical_bytes" false measurement else measurement
) Fixture.measurements) Fixture
| Contracts.Profile
