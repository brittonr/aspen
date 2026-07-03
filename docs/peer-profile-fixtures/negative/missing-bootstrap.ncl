let Contracts = import "../../peer-profile-contracts.ncl" in

{
  schema_id = Contracts.schema_id,
  schema_version = Contracts.schema_version,
  source_language = Contracts.source_language,
  profiles = [
    {
      peer_ref = "blake3:1111111111111111111111111111111111111111111111111111111111111111",
      endpoint = "iroh://peer-a",
      topic = "node-control",
      scope = "operator-read",
      resource_ref = "blake3:2222222222222222222222222222222222222222222222222222222222222222",
      bootstrap_refs = [],
      policy_refs = ["blake3:4444444444444444444444444444444444444444444444444444444444444444"],
      resource_refs = ["blake3:5555555555555555555555555555555555555555555555555555555555555555"],
    },
  ],
} | Contracts.peer_profile_export_contract
