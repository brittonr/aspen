let Contracts = import "../contracts.ncl" in
(import "../default.ncl") & {
  determinism_surface_inventory | force = [{
    id = "status_without_case_coverage",
    command = "status --root <root>",
    surface_kind = "cli",
    receipt_command = "status",
    required_replay_cases = ["missing_status_case"],
    required_replay_groups = ["smoke"],
    determinism_claim = "status_receipt_stability",
  }],
} | Contracts.Policy
