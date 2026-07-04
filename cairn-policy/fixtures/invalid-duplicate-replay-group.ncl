let Contracts = import "../contracts.ncl" in
let Base = import "../default.ncl" in
(Base & {
  determinism_replay_policy.groups | force =
    Base.determinism_replay_policy.groups @ [std.array.at 0 Base.determinism_replay_policy.groups],
}) | Contracts.Policy
