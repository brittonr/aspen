## Why

Cluster and VM failures can produce useful receipts and logs, but without a sealed failure bundle reviewers must reconstruct context from scattered artifacts. Failure evidence also needs privacy, redaction, and non-pass boundaries so diagnostic bundles cannot accidentally satisfy pass gates.

## What Changes

- Add sealed cluster failure repro bundles for denied, unavailable, or failed-validation cluster/local-multiprocess/VM runs.
- Bind scenario fixture refs, topology refs, command refs, node summaries, child receipts, diagnostics, diagnostic-log refs, redaction policy refs, private attachment refs, reveal receipts, replay status, and evidence-only caveats.
- Verify bundles fail closed on tampering, stale refs, missing redaction evidence, private content without reveal, and diagnostic-only pass-gate attempts.
- Support deterministic replay only when a recorded effect log exists; local multiprocess and VM bundles remain non-replayable diagnostics otherwise.

## Impact

Cluster failures become portable, reviewable artifacts without promoting logs or private data to authority. Bundles remain diagnostic-only and do not grant authority, policy, provenance, source-gate, resource, transport, retention, deployment, production, or pass evidence.
