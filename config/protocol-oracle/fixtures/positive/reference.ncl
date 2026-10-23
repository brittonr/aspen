# Exercises the non-default domain members the reference profile does not use.
let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(
  Base
  & {
    services
      | force
      = [
        {
          service = "transactional-key-value",
          logical_position = "version",
          safety = ["local", "pairwise", "cohort", "durability"],
          physical_layout = "canonical-physical",
        },
        {
          service = "replicated-log",
          logical_position = "next-offset",
          safety = ["pairwise"],
          physical_layout = "semantic-only",
        },
      ],
    participants
      | force
      = [
        { participant = "node-a", service = "transactional-key-value", is_required = true },
        { participant = "node-b", service = "transactional-key-value", is_required = false },
        { participant = "node-c", service = "replicated-log", is_required = true },
      ],
    completeness | force = "sampled",
    novelty_fields
      | force
      = [
        "service",
        "participant",
        "transition-class",
        "logical-position",
        "state",
        "durability",
        "progress",
      ],
    liveness.virtual_progress_horizon | force = 1000000,
    bounds
      | force
      = {
        max_records = 65536,
        max_state_bytes = 1048576,
        max_total_bytes = 1048576,
        max_participants = 3,
        max_counters = 4,
      },
  }
) | Contracts.Profile
