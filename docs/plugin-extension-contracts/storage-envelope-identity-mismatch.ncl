let Envelope = import "envelope.ncl" in
{
  schema_id = Envelope.grant_schema_id,
  schema_version = Envelope.schema_version,
  source_language = Envelope.source_language,
  export_identity = "plugin:storage/storage.write",
  payload = import "storage.grant.ncl",
} | Envelope.GrantEnvelope
