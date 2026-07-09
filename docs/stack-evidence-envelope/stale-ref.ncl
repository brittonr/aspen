let Contracts = import "contracts.ncl" in
let NonClaim = "evidence-only: no runtime authority or verifier soundness claim" in
let member = fun role_name artifact_ref_value => {
  role = role_name,
  schema = "molten.stack-evidence.member.v1",
  artifact_ref = artifact_ref_value,
  verification_role = "manual-review-input",
  non_claims = [NonClaim],
} in
{
  schema = "molten.stack-evidence.envelope.v1",
  members = [
    member "basalt" "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    member "ucan" "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    member "trellis" "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    member "octet" "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    member "valence" "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    member "cairn" "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    member "mantle" "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  ],
} | Contracts.Envelope
