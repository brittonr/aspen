## Implementation

- [x] [serial] Run the original schema tests and add failing pathless round-trip cases. Cite r[molten.toolchain.pathless_identity]. Evidence: `evidence/validation.md`.
- [x] [serial] Repair formatting and explicit-name parsing without changing source URLs. Cite r[molten.toolchain.pathless_identity]. Evidence: `evidence/validation.md`.
- [x] [serial] Cover malformed fragments, ordinary URLs, and Git reference preservation. Cite r[molten.toolchain.pathless_boundary]. Evidence: `evidence/validation.md`.
- [x] [serial] Build the pinned patched Cargo through Nix and retain the original compiler cohort. Cite r[molten.toolchain.pathless_boundary]. Evidence: `evidence/validation.md`.
- [ ] [serial] Verify actual locked all-feature metadata and nextest through the repaired toolchain. Cite r[molten.toolchain.pathless_acceptance].
- [ ] [serial] Run applicable format, Nix, lifecycle, and source review checks and bind the evidence. Cite r[molten.toolchain.pathless_acceptance].
