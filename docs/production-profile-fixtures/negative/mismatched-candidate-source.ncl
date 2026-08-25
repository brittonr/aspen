let CandidateSourceRef = "blake3:d54663d8d304af6d5f486ead64a51c06d74e1ab6320f62b2c6e9864dfe5f33c6" in
let OtherSourceRef = "blake3:07a363ec714438ebdf999afd3a87e6baab1b2072b99bd5867dbb481552c0494a" in
(import "../profile-template.ncl").make {
  candidate_source_ref = CandidateSourceRef,
  source_gate_inputs = [OtherSourceRef],
}
