let Contracts = import "../contracts.ncl" in
let Base = import "../default.ncl" in
(Base & {
  determinism_replay_policy.cases | force =
    Base.determinism_replay_policy.cases @ [std.array.at 0 Base.determinism_replay_policy.cases],
}) | Contracts.Policy
