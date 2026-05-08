# FastCDC Baseline Measurement Evidence

Status: captured
Date: 2026-05-08

## Scope

This evidence adds a reproducible baseline measurement harness for the current Aspen Snix FastCDC chunking implementation and captures one local baseline report. The harness is intentionally dependency-light and keeps the current production default unchanged.

## Implementation

- Added `crates/aspen-snix/benches/chunking_baseline.rs`.
- Added `[[bench]] name = "chunking_baseline"` to `crates/aspen-snix/Cargo.toml`.
- The harness measures the current `aspen_snix::chunking::chunk_blob` FastCDC path.
- The harness emits JSON with:
  - algorithm and chunk size parameters,
  - corpus class/file/byte counts,
  - per-file byte/chunk timings,
  - total chunks, unique chunks, duplicate chunks, dedup ratio,
  - chunk size min/p50/p95/max,
  - wall-clock microseconds and MiB/s throughput.

## Corpus

The baseline run used three bounded corpus classes:

1. `synthetic-small-delta`
   - Deterministic generated base plus small insertion/mutation variants.
   - Exercises CDC boundary stability and chunk reuse across related blobs.
2. `nix-store`
   - Bounded local regular files from `/run/current-system/sw/bin` and `/nix/store`.
   - Exercises real Nix store-derived content present on the runner.
3. `aspen-build-artifact`
   - Bounded local `target/{debug,release}` artifacts plus the benchmark binary itself.
   - Exercises Aspen build-output-like binaries/artifacts present on the runner.

The harness caps filesystem scanning with `MAX_FILES_PER_CLASS`, `MAX_BYTES_PER_FILE`, and `MAX_SCAN_DIR_ENTRIES` constants so the benchmark remains local and bounded.

## Captured baseline report

Report path: `openspec/changes/evaluate-vectorcdc-chunking/evidence/fastcdc-baseline-report.json`

Summary from the captured JSON:

- Algorithm: `fastcdc-v2020`
- Parameters:
  - min chunk size: `16384`
  - avg chunk size: `65536`
  - max chunk size: `262144`
  - inline threshold: `262144`
- Corpus:
  - files: `42`
  - bytes: `17087105`
  - classes:
    - `aspen-build-artifact`: `17` files, `2058081` bytes
    - `nix-store`: `16` files, `10295044` bytes
    - `synthetic-small-delta`: `9` files, `4733980` bytes
- Results:
  - wall time: `6405` microseconds
  - throughput: `2544.0583358291947` MiB/s
  - total chunks: `250`
  - unique chunks: `208`
  - duplicate chunks: `42`
  - dedup ratio: `0.168`
  - chunk size min/p50/p95/max: `14` / `68901` / `137643` / `262144`

## Commands / Oracles

- command: `nix run .#rustfmt`
- command: `cargo test -p aspen-snix chunking --lib`
- command: `cargo bench -p aspen-snix --bench chunking_baseline > openspec/changes/evaluate-vectorcdc-chunking/evidence/fastcdc-baseline-report.json`
- oracle: parse `fastcdc-baseline-report.json` as JSON and confirm all three corpus classes are present.

## Outcomes

- result: pass — the chunking unit tests still pass after adding the benchmark harness.
- result: pass — the baseline harness runs under Cargo bench and emits valid JSON.
- result: pass — the captured corpus covers synthetic small-delta content, real local Nix store-derived content, and Aspen build artifacts.
- result: pass — FastCDC baseline evidence now records throughput, wall time, chunk-size distribution, chunk count, reuse/dedup ratio, and corpus/object impact fields for future candidate comparison.
