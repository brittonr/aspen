let Contracts = import "contracts.ncl" in
let Ref = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in
let NonClaim = "evidence-only: no runtime authority or verifier soundness claim" in
let member = fun role_name => {
  role = role_name,
  schema = "molten.stack-evidence.member.v1",
  artifact_ref = Ref,
  verification_role = "manual-review-input",
  non_claims = [NonClaim],
} in
{
  schema = "molten.stack-evidence.envelope.v1",
  members = [
    member "basalt",
    member "ucan",
    member "trellis",
    member "octet",
    member "valence",
    member "cairn",
  ],
} | Contracts.Envelope
