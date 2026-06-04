## Context

Aspen's VM job submission moved from base64-in-JSON to blob refs, reducing bandwidth and enabling P2P distribution and dedup. Molten should generalize this for Wasm, native adapters, Steel scripts, distributed job DAG stages, and remote execution. Job submission is an actor/runtime operation, but large content moves through content-addressed storage.

## Goals

- Keep job envelopes small by referencing large executable/input/output content.
- Verify content refs before execution.
- Pin job inputs/executables during execution and retention windows.
- Cache fetched content by content id and chunk manifest.
- Make job execution deterministic under local/record/replay handler profiles.
- Emit receipts for submission, fetch, verification, admission, execution, result, and cleanup.

## Non-Goals

- Do not inline large binaries or datasets in normal envelopes.
- Do not trust content because a peer supplied it; verify hashes/manifests.
- Do not let blob refs bypass artifact provenance or effect admission.
- Do not make the job queue the default actor mailbox.

## Job payload model

A job submission should include:

- job id and operation id,
- executable artifact or chunk/blob manifest ref,
- input value or input content refs,
- size and format/media hints,
- schema refs for inputs/outputs,
- effect manifest refs,
- handler profile and resource grants,
- authority context,
- policy/provenance/evidence refs,
- expected result storage mode.

Large outputs should also be stored by content ref with receipts.

## Worker flow

1. Receive job envelope through dataspace or job service.
2. Validate authority, resource budget, and idempotency key.
3. Resolve executable/input refs.
4. Fetch missing chunks/blobs/artifacts.
5. Verify content and provenance.
6. Bind effect handlers.
7. Execute under selected profile.
8. Store output as inline value or content ref.
9. Emit result and cleanup receipts.

## Integration with SAM actors

Actors submit jobs by asserting demand or sending an admitted job request to a job service. Workers assert status: queued, fetching, running, complete, failed, cancelled, or result-ready. The job service may use stronger coordination primitives internally, but ordinary actor interaction remains dataspace-based.

## Security and retention

Executable refs need provenance policy. Secret inputs use secret refs or encrypted chunk refs. Job inputs/executables/results are pinned while active and retained according to policy. Deletion requires chunk-store/retention proof.

## Open Questions

- Which executable format should be first: Wasm component, Steel script, native test adapter, or VM binary?
- Should job queue state be local dataspace first or Raft-backed from the start?
- How should large stdout/stderr/log streams be chunked and redacted?
