# Verification Evidence

## Implementation Evidence

- Changed file: `openspec/changes/extract-blob-castore-cache/tasks.md`
- Changed file: `openspec/changes/extract-blob-castore-cache/verification.md`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/r1-capture-baseline.sh`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline.md`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-blob.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-castore.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-cache.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-rpc-core-blob.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-blob-handler.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-snix.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-snix-bridge.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-nix-cache-gateway.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-ci-executor-nix.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/r1-openspec-preflight.txt`

## Task Coverage

- [x] R1 Capture baseline compile and dependency graphs for `aspen-blob`, `aspen-castore`, `aspen-cache`, and representative consumers, classifying each dependency as backend-purpose, reusable domain, adapter/runtime, test-only, or forbidden. [covers=blob-castore-cache-extraction.castore-cache-avoid-app-shells,blob-castore-cache-extraction.blob-default-avoids-app-shells,blob-castore-cache-extraction.blob-default-avoids-app-shells.iroh-backend-is-documented-exception,blob-castore-cache-extraction.castore-cache-avoid-app-shells.castore-circuit-breaker-is-reusable-or-gated,blob-castore-cache-extraction.castore-cache-avoid-app-shells.cache-metadata-signing-reusable] ✅ completed: 2026-04-25T18:49:10Z
  - Evidence: `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline.md`, `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/`

## Review Scope Snapshot

R1 is evidence-only baseline capture. Source implementation diff begins with later tasks.

## Verification Commands

### `bash -n openspec/changes/extract-blob-castore-cache/evidence/r1-capture-baseline.sh`

- Status: pass
- Artifact: command completed without output before running the capture script.

### `./openspec/changes/extract-blob-castore-cache/evidence/r1-capture-baseline.sh`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline.md`

### `scripts/openspec-preflight.sh extract-blob-castore-cache`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/r1-openspec-preflight.txt`

## Notes

- `aspen-castore` baseline compile fails before implementation because the existing client passes `Instant` into the `aspen-core` circuit breaker API that now expects `u64` milliseconds. See `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-castore.txt`.
