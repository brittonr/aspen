let Prelude = import "../nickel-contract-prelude.ncl" in

let SchemaVersion = 1 in
let SourceLanguage = "nickel" in
let ContractSchemaId = "molten.plugin-extension.contract.export.v1" in
let GrantSchemaId = "molten.plugin-extension.grant.export.v1" in
let KnownSchemaId = Prelude.contract (fun value =>
  std.is_string value && (value == ContractSchemaId || value == GrantSchemaId)
) in
let ExactSchemaVersion = Prelude.contract (Prelude.is_exact_integer SchemaVersion) in
let ExactSourceLanguage = Prelude.contract (Prelude.is_exact_string SourceLanguage) in
let ExportEnvelopeContract = {
  schema_id | KnownSchemaId,
  schema_version | ExactSchemaVersion,
  source_language | ExactSourceLanguage,
  export_identity | Prelude.contracts.NonEmptyString,
  payload | Dyn,
} in
{
  schema_version = SchemaVersion,
  source_language = SourceLanguage,
  contract_schema_id = ContractSchemaId,
  grant_schema_id = GrantSchemaId,
  ExportEnvelope = ExportEnvelopeContract,
}
