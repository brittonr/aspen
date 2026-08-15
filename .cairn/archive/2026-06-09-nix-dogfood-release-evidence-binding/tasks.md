# Tasks: nix-dogfood-release-evidence-binding

- [x] [serial] r[molten.operator_dogfood_nix_release_evidence.export] Add canonical Nix dogfood release evidence that binds output path, report ref, release-gate ref, summary ref, nextest marker ref, and preserved file refs.
- [x] [serial] r[molten.operator_dogfood_nix_release_evidence.verify] Add verification receipts that recompute Nix dogfood output refs and deny mismatches before release review.
- [x] [serial] r[molten.operator_dogfood_nix_release_evidence.nix_check] Make the Nix dogfood check emit evidence and verification receipts into its output path.
- [x] [parallel] r[molten.operator_dogfood_nix_release_evidence.evidence_only] Document that Nix dogfood release evidence remains evidence-only and does not grant authority, policy, provenance, resource, transport, source-gate, retention, or destructive-operation trust.
- [x] [parallel] r[molten.operator_dogfood_nix_release_evidence.tests] Add CLI/unit coverage for export, verify, summaries, and Nix output binding.
