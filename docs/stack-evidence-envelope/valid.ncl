let Contracts = import "contracts.ncl" in
let Ref = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in
let NonClaim = "evidence-only: this input does not grant authority, prove runtime correctness, prove upstream verifier soundness, or grant deployment trust" in
let member = fun role_name schema_id => {
  role = role_name,
  schema = schema_id,
  artifact_ref = Ref,
  verification_role = "manual-review-input",
  non_claims = [NonClaim],
} in
{
  schema = "molten.stack-evidence.envelope.v1",
  members = [
    member "basalt" "molten.stack-evidence.basalt.v1",
    member "ucan" "molten.stack-evidence.ucan.v1",
    member "trellis" "molten.stack-evidence.trellis.v1",
    member "octet" "molten.stack-evidence.octet.v1",
    member "valence" "molten.stack-evidence.valence.v1",
    member "cairn" "molten.stack-evidence.cairn.v1",
    member "mantle" "molten.stack-evidence.mantle.v1",
  ],
} | Contracts.Envelope
