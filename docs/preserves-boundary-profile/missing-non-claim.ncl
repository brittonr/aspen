let Contracts = import "contracts.ncl" in
let Fixture = import "fixture.ncl" in
std.record.update "rows" (std.array.map (fun row =>
  if row.artifact_family == "node-control-envelope" then std.record.update "non_claims" [] row else row
) Fixture.rows) Fixture
| Contracts.Profile
