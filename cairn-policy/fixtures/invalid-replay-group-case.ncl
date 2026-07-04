let Contracts = import "../contracts.ncl" in
let Base = import "../default.ncl" in
(Base & {
  determinism_replay_policy.groups | force = Base.determinism_replay_policy.groups @ [{
    id = "unknown-case-group",
    description = "group references a missing replay case",
    cases = ["missing_replay_case"],
  }],
}) | Contracts.Policy
