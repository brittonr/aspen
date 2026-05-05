# Aspen operator diagnostics evidence contracts.
#
# Placeholder Rust-derived evidence-family boundary for promoted diagnostic DTOs.
# Rust owns DTO serialization; this Nickel file validates the documented common
# envelope shape used by generated evidence contracts and fixtures.

let contains = fun values value =>
  std.array.fold_left (fun found item => found || item == value) false values in

let enum_string = fun field allowed =>
  std.contract.from_validator (fun value =>
    if std.is_string value && contains allowed value then
      'Ok
    else
      'Error { message = "%{field} must be one of %{std.serialize 'Json allowed}" }
  ) in

let non_empty = fun field =>
  std.contract.from_validator (fun value =>
    if std.is_string value && std.string.length value > 0 then
      'Ok
    else
      'Error { message = "%{field} must be a non-empty string" }
  ) in

let Severity = enum_string "severity" [
  "info",
  "warning",
  "error",
  "fatal",
] in

let EvidenceArtifact = {
  name | non_empty "artifact.name",
  kind | non_empty "artifact.kind",
  path | String | optional,
  digest | String | optional,
  redacted | Bool | default = true,
} in

let DiagnosticFinding = {
  id | non_empty "finding.id",
  severity | Severity,
  summary | non_empty "finding.summary",
  remediation | String | optional,
  artifacts | Array EvidenceArtifact | default = [],
} in

let DiagnosticEnvelope = {
  schema_version | String | default = "1",
  generated_by | non_empty "generated_by",
  subject | non_empty "subject",
  findings | Array DiagnosticFinding | default = [],
  redaction_policy | String | default = "secret-values-forbidden",
} in

let SeverityContract = Severity in
let EvidenceArtifactContract = EvidenceArtifact in
let DiagnosticFindingContract = DiagnosticFinding in
let DiagnosticEnvelopeContract = DiagnosticEnvelope in
{
  Severity = SeverityContract,
  EvidenceArtifact = EvidenceArtifactContract,
  DiagnosticFinding = DiagnosticFindingContract,
  DiagnosticEnvelope = DiagnosticEnvelopeContract,
}
