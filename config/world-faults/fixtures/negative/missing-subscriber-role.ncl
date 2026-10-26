let Valid = import "../../profiles/local-deterministic.ncl" in
let Contracts = import "../../contracts.ncl" in
std.record.remove "subscriber" Valid | Contracts.Profile
