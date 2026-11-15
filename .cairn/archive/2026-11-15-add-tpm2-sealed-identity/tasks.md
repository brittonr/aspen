# Tasks

- [x] [serial] Implement explicit production backend eligibility and exact cross-backend rotation admission. r[molten.crypto_identity.tpm2_sealed_transport] r[molten.crypto_identity.tpm2_backend_rotation]
- [x] [serial] Implement capability-rooted bounded TPM object verification/unseal with typed denial and memory-only secret transport. r[molten.crypto_identity.tpm2_sealed_transport]
- [x] [serial] Add consumer tests and swtpm seal/load/PCR-denial/secrecy/rotation fixture. r[molten.crypto_identity.tpm2_sealed_transport] r[molten.crypto_identity.tpm2_backend_rotation]
- [x] [serial] Integrate the pinned Nix rail and run focused checks from the parent integration owner. r[molten.crypto_identity.tpm2_sealed_transport] r[molten.crypto_identity.tpm2_backend_rotation]

## Parent execution (2026-11-13)

Pure TPM controls passed two tests; the separately selected corrected swtpm
consumer passed its one real seal/load, rotation, secrecy and PCR-denial case.
The Nix identity-profile check also passed. The harness packaged the current
Cargo-built library test executable, not the complete hermetic Nix test-binary
output. `evidence/swtpm-2026-11-13/` retains only the actual consumer verdict and
pinned tool/provider versions; no seed or provider device state is published.

