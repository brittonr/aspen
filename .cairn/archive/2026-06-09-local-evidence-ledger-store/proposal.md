# Change: local-evidence-ledger-store

## Why

Molten can emit canonical reports, receipts, sealed bundles, and repro artifacts, but they are currently loose files. The next implementation stage needs a local evidence ledger that can index, verify, retain, and garbage-collect artifacts without losing the Preserves/hash rails.

## What

- Add a local content-addressed evidence ledger backed by Redb or equivalent embedded storage.
- Store canonical Preserves artifacts by hash and index them by evidence class, suite ref, report ref, bundle ref, signer ref, and creation/validation receipts.
- Make ledger writes append-only from the evidence perspective: new receipts supersede old status, but stored content bytes are immutable by hash.
- Add retention pins and GC eligibility for diagnostic failures, pass evidence, unpacked bundles, and distributed fetches.
- Provide CLI import/export/verify/list commands that preserve plain-file interoperability.

## Impact

This gives Molten a durable local substrate for receipts and repro bundles before introducing distributed exchange or production ledgers. It also makes test fixtures less dependent on ad hoc target/tmp paths.
