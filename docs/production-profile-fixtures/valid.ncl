# r[verify molten.prod_ops.production_profile.candidate_input]
# r[verify molten.prod_ops.production_profile.candidate_fixture_non_claim]
let FixtureCandidateSourceRef = "blake3:d54663d8d304af6d5f486ead64a51c06d74e1ab6320f62b2c6e9864dfe5f33c6" in
(import "profile-template.ncl").make {
  candidate_source_ref = FixtureCandidateSourceRef,
}
