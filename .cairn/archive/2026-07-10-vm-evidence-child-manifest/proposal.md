## Why

The VM check preserves a rich `vm-evidence/` tree, but the manifest should be the reviewable closure. If child receipts referenced by VM test-run, prod-soak, shard, or aggregate evidence are omitted from the manifest, reviewers must inspect directories manually and missing artifacts can hide behind top-level pass receipts.

## What Changes

- Include every child receipt and diagnostic log referenced by VM test-run, prod-soak, shard, aggregate, and validation receipts in `vm-evidence-manifest.preserves`.
- Add a manifest closure validator that checks referenced artifact refs, paths, kinds, duplicates, missing files, unreferenced required children, and diagnostic-only log boundaries.
- Preserve child manifests per shard and in the full aggregate output.

## Impact

Reviewers can inspect a complete VM evidence closure from one manifest. This improves auditability without promoting logs or manifests into authority, policy, provenance, or production trust.
