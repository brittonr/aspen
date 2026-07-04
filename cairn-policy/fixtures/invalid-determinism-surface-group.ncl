let Contracts = import "../contracts.ncl" in
let Base = import "../default.ncl" in
(Base & {
  determinism_surface_inventory | force = Base.determinism_surface_inventory @ [{
    id = "surface_with_unknown_group",
    command = "status --root <root>",
    surface_kind = "cli",
    receipt_command = "status",
    required_replay_cases = ["status"],
    required_replay_groups = ["missing_group"],
    determinism_claim = "status_receipt_stability",
  }],
}) | Contracts.Policy
