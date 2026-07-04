## Why

When a VM multinode check fails, the useful evidence is scattered across topology files, node evidence, child receipts, diagnostics, and VM logs. The repo already has failure repro bundle concepts, but VM failures should automatically produce sealed diagnostic bundles so reviewers can inspect and share the exact failed boundary without treating logs as pass evidence.

## What Changes

- Export a sealed diagnostic VM failure repro bundle when a VM shard or aggregate denies, is unavailable, or fails validation.
- Bind scenario fixture refs, topology refs, node summary refs, child receipt refs, diagnostic log refs, redaction policy refs, privacy markers, and replay status into the bundle.
- Add verify and pass-gate checks that reject tampered, private-without-reveal, stale, unsealed, and diagnostic-only bundles as pass evidence.
- Preserve enough data to reproduce deterministic simulation or local multiprocess failures when available, while marking VM/live observations non-replayable diagnostics.

## Impact

VM failures become actionable artifacts instead of raw log archaeology. Diagnostic bundles help triage regressions without weakening pass-evidence, privacy, or redaction boundaries.
