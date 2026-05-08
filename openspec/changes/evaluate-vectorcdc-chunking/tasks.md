## Phase 1: Spec Baseline

- [x] [serial] Create the OpenSpec baseline for VectorCDC-style CDC evaluation.
- [x] [serial] Inventory current Aspen Snix chunking entrypoints, constants, tests, and call sites to define the smallest chunker boundary. Evidence: `evidence/current-chunking-inventory.md`.

## Phase 2: Baseline Measurement

- [x] [depends:inventory] Add or select a representative benchmark corpus covering real Nix store files, Aspen build artifacts, and synthetic small-delta mutations. Evidence: `evidence/fastcdc-baseline.md` and `evidence/fastcdc-baseline-report.json`.
- [x] [depends:corpus] Capture FastCDC baseline evidence: CDC throughput, wall time, chunk-size distribution, chunk count, reuse/dedup ratio, and object/manifest impact. Evidence: `evidence/fastcdc-baseline.md` and `evidence/fastcdc-baseline-report.json`.

## Phase 3: Candidate Evaluation

- [ ] [depends:baseline] Prototype one VectorCDC-style or hashless CDC candidate behind an explicit feature/config gate without changing default behavior.
- [ ] [depends:candidate] Run the same benchmark suite for the candidate and compare throughput, dedup ratio, boundary stability, portability, and end-to-end ingest costs.
- [ ] [depends:comparison] Record an adoption decision: keep FastCDC, tune FastCDC bounds, continue research, or open a separate promotion OpenSpec.

## Phase 4: Verification

- [ ] [depends:decision] Add positive and negative tests for chunk invariants, default dependency isolation, unsupported-candidate handling, and raw blob digest preservation.
- [ ] [depends:tests] Run focused Snix/chunking verification and OpenSpec validation before archive.
