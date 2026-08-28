let Valid = import "../../profiles/local-deterministic.ncl" in
let Contracts = import "../../contracts.ncl" in
(
  Valid
  & {
    schedules | force =
      std.array.map
        (fun schedule => if schedule.mutation == "head" then schedule & { wall_clock_timing | force = true } else schedule)
        Valid.schedules,
  }
) | Contracts.Profile
