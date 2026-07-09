let Contracts = import "contracts.ncl" in
let Ref = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in
let NonClaim = "evidence-only: no runtime authority or verifier soundness claim" in
let member = fun role_name claim_list => {
  role = role_name,
  schema = "molten.stack-evidence.member.v1",
  artifact_ref = Ref,
  verification_role = "manual-review-input",
  non_claims = claim_list,
} in
{
  schema = "molten.stack-evidence.envelope.v1",
  members = [
    member "basalt" ["grants authority to execute runtime effects"],
    member "ucan" [NonClaim],
    member "trellis" [NonClaim],
    member "octet" [NonClaim],
    member "valence" [NonClaim],
    member "cairn" [NonClaim],
    member "mantle" [NonClaim],
  ],
} | Contracts.Envelope
