let Revision = "3f7b4315c8e1d07726446f1e53ba45bc091c5275" in
let Adapter = "https://seed.radicle.garden/z1C4YVMgDGyVdQa72uPNj3UDS5cY.git" in
let SourceContract = {
  schema | String,
  repository_id | String,
  adapter | String,
  reviewed_revision | String,
  cargo_revision | String,
  nix_revision | String,
  executes_migrations | Bool,
  non_claims | Array String,
} in
{
  schema = "molten.schema-migration-pilot.v1",
  repository_id = "rad:z1C4YVMgDGyVdQa72uPNj3UDS5cY",
  adapter = Adapter,
  reviewed_revision = Revision,
  cargo_revision = Revision,
  nix_revision = Revision,
  executes_migrations = false,
  non_claims = [
    "planning does not prove migration execution",
    "planning does not grant runtime or storage authority",
  ],
} | SourceContract
