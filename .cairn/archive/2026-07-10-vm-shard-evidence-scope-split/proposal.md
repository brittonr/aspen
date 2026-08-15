## Why

VM shard checks can represent fixture metadata, executable VM observations, or aggregate evidence. When synthetic shard receipts and executable VM receipts share similar surfaces, reviewers can mistake bounded metadata evidence for platform execution evidence.

## What Changes

- Split synthetic shard metadata receipts from executable VM shard receipts in naming, caveats, and aggregate gates.
- Require aggregates to preserve each child shard's evidence scope instead of promoting metadata-only shards to platform pass evidence.
- Add diagnostics for log-only shard success, synthetic-ref-only pass claims, and executable claims without real VM child receipts.
- Document which checks are metadata/readback evidence and which checks are executable NixOS VM evidence.

## Impact

VM review becomes less ambiguous. Synthetic metadata remains useful for fixture/profile wiring, while platform claims require executable VM receipts. No shard receipt grants authority, policy, provenance, source-gate, resource, retention, or production trust.
