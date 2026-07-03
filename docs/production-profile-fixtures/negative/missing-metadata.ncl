let Contracts = import "../../production-profile-contracts.ncl" in
{
  profile_identity = "pilot-node",
  schema_version = Contracts.schema_version,
  source_language = Contracts.source_language,
  profile = (import "../profile-template.ncl").make {}.profile,
} | Contracts.ProductionProfileExport
