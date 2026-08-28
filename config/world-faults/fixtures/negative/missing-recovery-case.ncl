let Valid = import "../../profiles/local-deterministic.ncl" in
let Contracts = import "../../contracts.ncl" in
(
  Valid
  & {
    cases | force =
      std.array.filter
        (fun case => case.case_id != "capture:recovery-read-back")
        Valid.cases,
  }
) | Contracts.Profile
