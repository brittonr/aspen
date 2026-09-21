let Contracts = import "../../contracts.ncl" in
{
  schema_id = "molten.marble-store-pilot.catalog.v1",
  dependencies = [
    {
      crate = "marble",
      pinned_version = "16.0.2",
      transport = "git",
      plane = "implementation",
      license = "Apache-2.0",
      consumed_claims = [],
    },
    {
      crate = "art",
      pinned_version = "1.0.0",
      transport = "crates.io",
      plane = "implementation",
      license = "Apache-2.0",
      consumed_claims = [],
    },
  ],
  non_claims = [
    "spike-is-not-default-path",
    "object-id-is-not-content-identity",
    "measurement-is-not-proof",
    "recovery-claims-stop-at-recovered-batches",
  ],
} | Contracts.Export
