## Context

`src/provenance.rs` defines canonical `provenance-record-v1` and `provenance-receipt-v1` values. Node-control install and run dispatch already require admitted provenance for payload/job refs, but the only easy fixture command lived under `molten node`.

## Goals

- Expose provenance record construction under `molten test provenance record`.
- Expose a synthetic reviewed fixture under `molten test provenance fixture` for deterministic local tests.
- Expose provenance trust-state evaluation under `molten test provenance evaluate` using explicit provenance record files.
- Expose read-only summaries for records and receipts under `molten test provenance show`.
- Preserve fail-closed validation for malformed refs, unsupported trust states, unsupported profiles, mismatched artifacts, missing provenance, and sandbox-only records in node-control profiles.

## Non-Goals

- No new trust states or provenance policy semantics.
- No reproducible build execution or attestation verification beyond the current record fields.
- No node-control mutation from the provenance CLI.
- No authority, policy, resource, transport, execution, or source-gate grant from a provenance receipt.

## CLI Shape

```sh
molten test provenance fixture \
  --artifact-ref blake3:artifact \
  --out target/provenance.reviewed.preserves

molten test provenance record \
  --artifact-ref blake3:artifact \
  --trust-state reviewed \
  --source-ref blake3:source \
  --dependency-closure-ref blake3:deps \
  --toolchain-ref blake3:toolchain \
  --builder-ref blake3:builder \
  --review-ref blake3:review \
  --test-ref blake3:tests \
  --source-gate-ref blake3:octet \
  --policy-ref blake3:policy \
  --out target/provenance.record.preserves

molten test provenance evaluate \
  --operation install \
  --profile node-control \
  --artifact-ref blake3:artifact \
  --provenance target/provenance.record.preserves \
  --receipt-out target/provenance.receipt.preserves

molten test provenance show target/provenance.receipt.preserves
```

## Evidence Boundary

All command outputs are canonical Preserves records. Text summaries are non-normative diagnostics. Evaluation reads only explicit provenance files and writes only the requested receipt output.
