let Envelope = import "envelope.ncl" in
{
  schema_id = Envelope.contract_schema_id,
  schema_version = Envelope.schema_version,
  source_language = "yaml",
  export_identity = "plugin-extension:storage@1.0.0",
  payload = import "storage.ncl",
} | Envelope.ExportEnvelope
