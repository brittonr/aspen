let Contracts = import "../contracts.ncl" in
(import "../default.ncl") & {
  traceability_policy.profiles = [{
    id = "bad-traceability",
    requirement_sources = [
      { root = "../requirements", source_kind = "accepted_specs", extensions = ["md"] },
    ],
    evidence_sources = [
      { root = "src", source_kind = "ambient_source", extensions = ["rs"] },
    ],
    marker_verbs = ["claims"],
    assurance_level = "declared",
    anchor_kind = "exact_marker_bytes",
  }],
} | Contracts.Policy
