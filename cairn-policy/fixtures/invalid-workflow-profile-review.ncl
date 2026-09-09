# Negative fixture: review obligations must include all typed fields.
let Policy = import "../contracts.ncl" in
let Base = import "../default.ncl" in

(Base & {
  schemas = std.array.map (fun schema =>
    if schema.id == "assurance" then
      schema & {
        review_obligations = [{
          id = "assurance-plan-review",
          artifact_id = "assurance-review",
          review_kind = "plan",
          accepted_decisions = ["approved"],
          required_paths = ["proposal.md"],
        }],
      }
    else
      schema
  ) Base.schemas,
}) | Policy
