# Design: CI receipt artifact evidence

## Receipt Shape

Each `CiRunReceiptJob` gains an `artifacts` list containing the same operator-safe `CiArtifactInfo` metadata already returned by `CiListArtifacts`: blob hash, artifact name, size, content type, creation timestamp, and extra metadata. The receipt does not include blob tickets because tickets are download authorities and should remain explicit follow-up results from `CiGetArtifact`.

## Collection Path

`CiGetRunReceipt` already resolves the run through `PipelineOrchestrator::get_run`. Receipt rendering will also receive the CI KV store. For every job with an Aspen job ID, it scans `_ci:artifacts:{job_id}:` with a bounded limit and filters metadata through the existing artifact metadata schema. Jobs without IDs get an empty artifact list.

## Determinism and Bounds

- Jobs remain sorted by job name.
- Artifacts are sorted by artifact name, then blob hash.
- Each job artifact scan is bounded to the existing per-job artifact limit of 100 records.
- The receipt request fails explicitly if an artifact scan fails, avoiding a misleading partial receipt.

## Compatibility

The change extends the native CI run receipt payload. The receipt schema remains operator-facing JSON with `schema = aspen.ci.run-receipt.v1`; new consumers can rely on `artifacts`, while existing follow-up commands remain unchanged.

## Verification Strategy

- Add/extend handler tests showing receipt jobs include deterministic artifact metadata.
- Verify CLI output reports artifact count for human receipt output while JSON remains machine-readable.
- Run focused CI handler/client API/CLI tests and OpenSpec validation.
