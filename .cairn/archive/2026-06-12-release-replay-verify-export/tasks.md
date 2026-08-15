# Tasks: release-replay-verify-export

- [x] [serial] r[molten.release.replay_verify_export.local_output] Emit raw deterministic replay verify receipts from local dogfood outputs.
- [x] [serial] r[molten.release.replay_verify_export.readback] Bind replay verify refs into Nix dogfood release evidence and verification receipts.
- [x] [serial] r[molten.release.replay_verify_export.bundle] Include replay verify members in release bundle and signed-member verification paths.
- [x] [serial] r[molten.release.replay_verify_export.archive] Include replay verify members in release export manifests and archive verification.
- [x] [parallel] r[molten.release.replay_verify_export.tests] Cover dogfood output, readback, signed bundle, export, and evidence-only behavior.
