## Tasks

- [x] [serial] Freeze the clean candidate commit, Git tree, and framed BLAKE3 source identity. r[molten.prod_release.pilot_candidate_freeze]
- [x] [serial] Run fresh Rust, nextest, Nix, Cairn, and Octet validation from the frozen candidate. r[molten.prod_release.pilot_evidence_publication]
- [x] [serial] Run fresh VM, dogfood, signed bundle, promotion, and export validation from the frozen candidate. r[molten.prod_release.pilot_evidence_publication]
- [x] [serial] Generate candidate-bound profile, pilot-decision, release-candidate, positive, and denial receipts. r[molten.prod_release.pilot_evidence_publication]
- [x] [serial] Publish scoped release notes and replace stale README evidence with the frozen candidate record. r[molten.prod_release.pilot_non_claims]
- [x] [serial] Pass strict Cairn gates, sync, archive, commit, integrate, tag, and verify remote publication. r[molten.prod_release.pilot_candidate_freeze] r[molten.prod_release.pilot_evidence_publication] r[molten.prod_release.pilot_non_claims]
