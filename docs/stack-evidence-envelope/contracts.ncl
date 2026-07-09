let Prelude = import "../nickel-contract-prelude.ncl" in
let RequiredRoles = ["basalt", "ucan", "trellis", "octet", "valence", "cairn", "mantle"] in
let EnvelopeSchema = "molten.stack-evidence.envelope.v1" in
let MemberSchemaPattern = "^molten\\.stack-evidence\\.[A-Za-z0-9_.-]+\\.v1$" in
let EvidenceOnlyMarker = "evidence-only" in
let OverbroadAuthorityPattern = "grants authority" in
let Role = Prelude.contract (Prelude.is_one_of RequiredRoles) in
let EnvelopeSchemaContract = Prelude.contract (Prelude.is_exact_string EnvelopeSchema) in
let MemberSchema = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match MemberSchemaPattern value
) in
let NonClaims = Prelude.contract (fun claims =>
  std.is_array claims
  && std.array.all std.is_string claims
  && std.array.any (fun claim => std.string.is_match EvidenceOnlyMarker claim) claims
  && std.array.all (fun claim => !std.string.is_match OverbroadAuthorityPattern claim) claims
) in
let MemberContract = {
  role | Role,
  schema | MemberSchema,
  artifact_ref | Prelude.contracts.Blake3Ref,
  verification_role | Prelude.contracts.NonEmptyString,
  non_claims | NonClaims,
} in
let role_set = fun members => std.array.map (fun member => member.role) members in
let complete_roles = fun members =>
  Prelude.has_all (role_set members) RequiredRoles
  && Prelude.distinct_strings (role_set members) in
let EnvelopeContract = std.contract.Sequence [{
  schema | EnvelopeSchemaContract,
  members | std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array MemberContract],
}, Prelude.contract (fun envelope => complete_roles envelope.members)] in
{
  RequiredRoles = RequiredRoles,
  Member = MemberContract,
  Envelope = EnvelopeContract,
}
