# The time-sliced component execution profile.
#
# It differs from the base component profile only in its identity and its
# slicing binding: a scheduling quantum and an independent total execution
# budget. The existing component profile is untouched.

let Template = import "profile-template.ncl" in
Template.make {
  profile_id = "molten.wasm.component.time-sliced.v1",
  slicing = {
    quantum_fuel = 1000,
    total_budget_fuel = 100000,
  },
}
