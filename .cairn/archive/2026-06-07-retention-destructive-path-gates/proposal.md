## Why

Retention records now model safe deletion, but several destructive paths still remove or tombstone local content through subsystem-specific receipts only. Ledger GC, chunk-store GC, evaluation-cache invalidation, and secret cleanup must all bind retention decisions before side effects.

## What Changes

- Gate evidence-ledger GC through per-artifact retention receipts before content removal.
- Gate chunk manifest/chunk GC through retention receipts before file removal and tombstone receipts.
- Gate evaluation-cache invalidation through retention receipts before cache tombstones are written.
- Require secret cleanup receipts to bind actual passing retention receipts for the target secret/tombstone.
- Surface retention receipt refs in subsystem receipts and CLI diagnostics.

## Impact

Destructive local maintenance becomes fail-closed and auditable. Retention receipts remain deletion-safety evidence only and do not grant authority, provenance, policy, transport, source-gate, resource, or execution trust.
