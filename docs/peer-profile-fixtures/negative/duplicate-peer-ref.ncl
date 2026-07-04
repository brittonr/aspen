let Contracts = import "../../peer-profile-contracts.ncl" in
let Valid = import "../valid.ncl" in
(Valid & {
  profiles | force = Valid.profiles @ Valid.profiles,
}) | Contracts.peer_profile_export_contract
