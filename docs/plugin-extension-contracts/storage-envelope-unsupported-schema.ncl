let Envelope = import "envelope.ncl" in
{
  schema_id = "molten.plugin-extension.unknown-export.v1",
  schema_version = Envelope.schema_version,
  source_language = Envelope.source_language,
  export_identity = "plugin-extension:storage@1.0.0",
  payload = import "storage.ncl",
} | Envelope.ExportEnvelope
