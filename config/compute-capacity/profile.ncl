let Contracts = import "contracts.ncl" in
{
  schema = "molten.compute.capacity-profile.v1",
  profile_id = "synthetic-two-host-capacity",
  profile_version = 1,
  maximum_items = 128,
  maximum_text_bytes = 256,
  time_domain = "logical",
  observation_absence_means_zero = false,
  live_qualification = false,
  backend_enforcement = false,
  non_claims = Contracts.required_non_claims,
} | Contracts.Profile
