let Base = import "../../ledger.ncl" in
let Contracts = import "../../contracts.ncl" in
(
  Base
  & {
    rows | force =
      std.array.map
        (fun row => if row.id == "multi-file-write" then row & { issue | force = null } else row)
        Base.rows,
  }
) | Contracts.Profile
