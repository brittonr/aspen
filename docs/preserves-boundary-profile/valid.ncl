# r[verify molten.runtime_spine.preserves_boundary_profile.contract]
# r[impl molten.runtime_spine.preserves_boundary_profile.fixtures.positive]
# r[verify molten.runtime_spine.preserves_boundary_profile.validation]
let Contracts = import "contracts.ncl" in
(import "fixture.ncl") | Contracts.Profile
