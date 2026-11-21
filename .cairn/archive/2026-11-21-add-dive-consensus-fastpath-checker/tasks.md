## Phase 1: Implementation

- [x] [serial] Confirm the release asset and tag against GitHub, pin the archive and version, preserve the MIT notice, and wire the exact provider into the existing Nix check. r[molten.consensus.fast_path_model.dive_parity]
- [x] [serial] Admit schema-3 reports and enforce three-way parity against TLC within existing bounds; retain byte-exact streams, reports and digests. r[molten.consensus.fast_path_model.dive_parity]
- [x] [serial] Add synthetic reader cases and document the checker pin, fail-closed conditions and finite-model-only claim. r[molten.consensus.fast_path_model.dive_parity]

## Phase 2: Verification

- [x] [serial] Run the synthetic reader tests, including refusal, stop, version or pin drift, timeout, incomplete reports, count divergence and wrong mutant property. r[molten.consensus.fast_path_model.dive_parity]
- [x] [serial] Run the existing x86_64 Linux Nix parity check and inspect the six observations, authoritative TLC counts, mutant verdicts, raw reports and BLAKE3 bindings. Confirm model, configs and bounds are unchanged. r[molten.consensus.fast_path_model.dive_parity]
- [x] [serial] Retain the actual execution evidence for review and native Cairn sync/archive. r[molten.consensus.fast_path_model.dive_parity]

See the [verified three-checker run](evidence/verification.md) and its exact retained output.
