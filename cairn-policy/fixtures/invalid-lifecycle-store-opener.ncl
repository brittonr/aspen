let Contracts = import "../contracts.ncl" in
(import "../default.ncl") & {
  lifecycle_store_policy.opener_profiles = [{
    id = "unsafe",
    executable = "sh -c",
    fixed_args = ["${member}"],
    member_argument_mode = "positional_paths",
    primary_first = true,
  }],
} | Contracts.Policy
