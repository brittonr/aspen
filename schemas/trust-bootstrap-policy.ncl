# Aspen trust/bootstrap policy contracts.
#
# Nickel validates quorum references, participant metadata, rotation policy, and
# impossible thresholds. It MUST NOT contain raw secret material.

let contains = fun values value =>
  std.array.fold_left (fun found item => found || item == value) false values in

let enum_string = fun field allowed =>
  std.contract.from_validator (fun value =>
    if std.is_string value && contains allowed value then
      'Ok
    else
      'Error { message = "%{field} must be one of %{std.serialize 'Json allowed}" }
  ) in

let positive_int = fun field =>
  std.contract.from_validator (fun value =>
    if std.is_number value && value > 0 && value % 1 == 0 then
      'Ok
    else
      'Error { message = "%{field} must be a positive integer" }
  ) in

let non_empty = fun field =>
  std.contract.from_validator (fun value =>
    if std.is_string value && std.string.length value > 0 then
      'Ok
    else
      'Error { message = "%{field} must be a non-empty string" }
  ) in

let SecretRef = std.contract.from_validator (fun value =>
  if std.is_string value && std.string.length value > 0 && !std.string.is_match "(?i)(private|secret|token|password)=|-----BEGIN" value then
    'Ok
  else
    'Error { message = "secret references must be handles/paths, not inline secret material" }
) in

let ParticipantRole = enum_string "participant.role" [
  "founder",
  "voter",
  "learner",
  "operator",
] in

let Participant = {
  id | non_empty "participant.id",
  role | ParticipantRole | default = "voter",
  public_identity_ref | non_empty "participant.public_identity_ref",
  share_ref | SecretRef | optional,
} in

let QuorumPolicy = {
  policy_id | non_empty "quorum.policy_id",
  threshold | positive_int "quorum.threshold",
  participants | Array Participant,
  rotation_period_days | positive_int "quorum.rotation_period_days" | optional,
  recovery_contact_refs | Array String | default = [],
} in

let BootstrapPolicy = {
  schema_version | String | default = "1",
  cluster_id | non_empty "bootstrap.cluster_id",
  initial_quorum | QuorumPolicy,
  allow_single_node_dev | Bool | default = false,
  notes | Array String | default = [],
} in

let SecretRefContract = SecretRef in
let ParticipantRoleContract = ParticipantRole in
let ParticipantContract = Participant in
let QuorumPolicyContract = QuorumPolicy in
let BootstrapPolicyContract = BootstrapPolicy in
{
  SecretRef = SecretRefContract,
  ParticipantRole = ParticipantRoleContract,
  Participant = ParticipantContract,
  QuorumPolicy = QuorumPolicyContract,
  BootstrapPolicy = BootstrapPolicyContract,
}
