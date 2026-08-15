## Why

Molten jobs and remote executions should not inline binaries, modules, datasets, or large inputs in envelopes. Aspen's VM job submission pattern is useful prior art: upload binary bytes to a content-addressed blob store, submit a job carrying only hash/size/format, and let workers fetch and verify content on demand.

## What Changes

- Define job submissions by artifact/blob/chunk refs instead of inline large bytes.
- Require job payloads to carry content refs, size, format/media type, schema refs, effect manifests, handler profile, and evidence refs.
- Workers fetch missing content through the chunk/blob store, verify hashes/manifests, then execute only after policy admission.
- Support content-addressed caching, dedup, resumable fetch, and retention pins for job inputs and executables.
- Integrate with distributed job DAGs, remote artifact sync, typed storage, deterministic playback, and operator receipts.

## Impact

This keeps envelopes small and makes job execution reproducible. The first milestone can submit a local job whose executable/input are manifest refs, fetch them from the local chunk/blob store, verify, run under a deterministic handler profile, and emit receipts.
