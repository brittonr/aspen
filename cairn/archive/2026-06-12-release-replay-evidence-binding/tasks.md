# Tasks: release-replay-evidence-binding

- [x] [serial] r[molten.release.replay_index_binding.gate] Bind replay evidence index refs into dogfood release gate receipts.
- [x] [serial] r[molten.release.replay_index_binding.readback] Make Nix dogfood and release bundle readback deny missing, stale, or tampered replay index evidence.
- [x] [serial] r[molten.release.replay_index_binding.bundle] Include replay index Preserves and signed members in release bundle signing/export paths.
- [x] [serial] r[molten.release.replay_index_binding.catalog] Classify release artifacts that bind replay indexes for catalog and replay MCP discovery.
- [x] [parallel] r[molten.release.replay_index_binding.tests] Cover dogfood replay index emission, stale/tampered replay index denial, signed bundle requirements, and evidence-only semantics.
