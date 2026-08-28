let Prelude = import "../../docs/nickel-contract-prelude.ncl" in
let ExpectedObligationCount = 7 in
let ExpectedOpenFormalCount = 1 in
let Status = Prelude.contract (Prelude.is_one_of ["model-checked", "open-formal"]) in
let Obligation = {
  id | Prelude.contracts.NonEmptyString,
  target_owner | Prelude.contract (Prelude.is_exact_string "trellis"),
  statement | Prelude.contracts.NonEmptyString,
  assumptions | std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array Prelude.contracts.NonEmptyString],
  status | Status,
  proves_database_correctness | Bool,
  proves_collision_impossibility | Bool,
}
in
let invariants = fun profile =>
  std.array.length profile.obligations == ExpectedObligationCount
  && Prelude.non_empty_distinct_by (fun item => item.id) profile.obligations
  && std.array.length (std.array.filter (fun item => item.status == "open-formal") profile.obligations) == ExpectedOpenFormalCount
  && std.array.all
    (fun item => item.proves_database_correctness == false && item.proves_collision_impossibility == false)
    profile.obligations
  && profile.formal_refinement_complete == false
  && profile.claims_database_correctness == false
  && profile.claims_collision_impossibility == false
in
{
  Proof =
    std.contract.Sequence [
      {
        schema | Prelude.contract (Prelude.is_exact_string "molten.prolly-map-proof-obligations.v1"),
        trellis_revision | Prelude.contract (fun value => std.is_string value && std.string.is_match "^[0-9a-f]{40}$" value),
        obligations | Array Obligation,
        formal_refinement_complete | Bool,
        claims_database_correctness | Bool,
        claims_collision_impossibility | Bool,
      },
      Prelude.contract invariants,
    ],
}
