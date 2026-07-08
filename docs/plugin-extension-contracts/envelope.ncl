let Prelude = import "../nickel-contract-prelude.ncl" in
let PluginExtensionContract = import "contract.ncl" in
let PluginCapabilityGrant = import "grant.ncl" in

let SchemaVersion = 1 in
let SourceLanguage = "nickel" in
let ContractSchemaId = "molten.plugin-extension.contract.export.v1" in
let GrantSchemaId = "molten.plugin-extension.grant.export.v1" in
let KnownSchemaId = Prelude.contract (fun value =>
  std.is_string value && (value == ContractSchemaId || value == GrantSchemaId)
) in
let ExactSchemaVersion = Prelude.contract (Prelude.is_exact_integer SchemaVersion) in
let ExactSourceLanguage = Prelude.contract (Prelude.is_exact_string SourceLanguage) in
let expected_contract_identity = fun payload => payload.extension_id ++ "@" ++ payload.version in
let expected_grant_identity = fun payload => payload.plugin_id ++ "/" ++ payload.operation in
let ContractExportIdentity = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match "^plugin-extension:[A-Za-z0-9_.:-]+@[0-9]+\\.[0-9]+\\.[0-9]+$" value
) in
let GrantExportIdentity = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match "^plugin:[A-Za-z0-9_.:-]+/[a-z][A-Za-z0-9_.:-]+$" value
) in
let ExportEnvelopeContract = {
  schema_id | KnownSchemaId,
  schema_version | ExactSchemaVersion,
  source_language | ExactSourceLanguage,
  export_identity | Prelude.contracts.NonEmptyString,
  payload | Dyn,
} in
let ContractEnvelopeContract = std.contract.Sequence [{
  schema_id | Prelude.contract (Prelude.is_exact_string ContractSchemaId),
  schema_version | ExactSchemaVersion,
  source_language | ExactSourceLanguage,
  export_identity | ContractExportIdentity,
  payload | PluginExtensionContract,
}, Prelude.contract (fun envelope => envelope.export_identity == expected_contract_identity envelope.payload)] in
let GrantEnvelopeContract = std.contract.Sequence [{
  schema_id | Prelude.contract (Prelude.is_exact_string GrantSchemaId),
  schema_version | ExactSchemaVersion,
  source_language | ExactSourceLanguage,
  export_identity | GrantExportIdentity,
  payload | PluginCapabilityGrant,
}, Prelude.contract (fun envelope => envelope.export_identity == expected_grant_identity envelope.payload)] in
{
  schema_version = SchemaVersion,
  source_language = SourceLanguage,
  contract_schema_id = ContractSchemaId,
  grant_schema_id = GrantSchemaId,
  ExportEnvelope = ExportEnvelopeContract,
  ContractEnvelope = ContractEnvelopeContract,
  GrantEnvelope = GrantEnvelopeContract,
}
