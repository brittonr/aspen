let Contracts = import "../../peer-profile-contracts.ncl" in
let Valid = import "../valid.ncl" in
(Valid & {
  profiles | force = [{
    peer_ref = "sha256:not-a-peer-ref",
    endpoint = "iroh://peer-a",
    topic = "node-control",
    scope = "operator-read",
    resource_ref = "blake3:2222222222222222222222222222222222222222222222222222222222222222",
    bootstrap_refs = ["blake3:3333333333333333333333333333333333333333333333333333333333333333"],
    policy_refs = ["blake3:4444444444444444444444444444444444444444444444444444444444444444"],
    resource_refs = ["blake3:5555555555555555555555555555555555555555555555555555555555555555"],
  }],
}) | Contracts.peer_profile_export_contract
