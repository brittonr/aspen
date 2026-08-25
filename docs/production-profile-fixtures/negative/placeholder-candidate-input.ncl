let ZeroCandidateSourceRef = "blake3:0000000000000000000000000000000000000000000000000000000000000000" in
(import "../profile-template.ncl").make {
  candidate_source_ref = ZeroCandidateSourceRef,
}
