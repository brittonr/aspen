# Design

Place `validate_profile(profile)` as the first operation in `load_prolly_snapshot`. The validator owns the full structural and identity contract, including `MAX_GRAPH_FACTS`; using it avoids a divergent shell constant or partial limit check. Return `ProllyServiceError::Domain(issues)` on failure. Only validated profiles reach `pending`, `seen`, `blocks`, or `port.read_block`. Retain `validate_snapshot` after the load for root, block, and closure checks; do not modify storage adapters or valid-limit behavior.
