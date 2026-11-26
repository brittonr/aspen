# r[verify molten.runtime_spine.preserves_boundary_profile.validation.core_coupling]
let Contracts = import "contracts.ncl" in
let Fixture = import "fixture.ncl" in
std.record.update "measurements" (std.array.map (fun measurement =>
  if measurement.artifact_family == "receipt" then std.record.update "raw_preserves_core_coupling" true measurement else measurement
) Fixture.measurements) Fixture
| Contracts.Profile
