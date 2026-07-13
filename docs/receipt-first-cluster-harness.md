# Receipt-first cluster harness

The receipt-first cluster harness runs a checked cluster manifest through isolated local node processes and makes the durable run directory—not terminal output—the review surface.

```sh
cargo run -- cluster harness-run \
  --fixture tests/fixtures/cluster-harness/two-node.cluster \
  --state-root target/cluster-harness-state \
  --run-dir target/cluster-harness-run

cargo run -- cluster harness-verify \
  --run-dir target/cluster-harness-run
```

`--child-timeout-ms` is explicit and bounded. Existing state or run directories are denied unless `--force` is supplied. Each node receives an isolated state root and logical local-process transport handle. This tier exercises real child-process startup, bounded workflow, status, reverse-order shutdown, and cleanup; it is not VM, live-network, consensus, or production evidence.

## Run directory

`artifact-index.tsv` binds every indexed artifact path to an explicit kind, format, and BLAKE3 content ref. The directory includes:

- fixture metadata and the derived command/process plan;
- canonical child-process and node lifecycle receipts;
- the local executable-run receipt;
- one canonical parent cluster-run receipt;
- a canonical drift summary and cleanup receipt;
- diagnostic child logs, explicitly classified as adjunct text;
- `verification.preserves`, the offline verification result.

`cluster harness-verify` needs no running child and fails closed on missing files, path traversal, malformed or non-canonical Preserves, kind drift, content-ref mismatch, denied child artifacts, missing required artifact kinds, unexpected files, or a stale verification companion. Its first-divergence field is diagnostic-only and identifies the earliest indexed mismatch.

## Failure handling

The process shell attempts bounded cleanup after failures, records timeout/orphan/ticket observations, and denies the parent receipt when required evidence is absent. A denied run may include `failure-repro-bundle.preserves` and `failure-repro-verification.preserves`. The sealed canonical bundle binds fixture, plan, child, lifecycle, receipt, log, redaction, and non-replayable local-observation refs; failure-bundle pass gates still reject it as pass evidence. Logs remain diagnostic-only and private attachments require the existing reveal/redaction policy before export.
