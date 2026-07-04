let Envelope = import "envelope.ncl" in
{
  schema_id = Envelope.contract_schema_id,
  schema_version = Envelope.schema_version,
  source_language = Envelope.source_language,
  payload = import "storage.ncl",
} | Envelope.ExportEnvelope
