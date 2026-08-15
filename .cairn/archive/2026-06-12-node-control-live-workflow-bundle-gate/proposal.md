# Change: Node Control Live Workflow Bundle Gate UX

## Motivation

Bundle verification gives operators an offline receipt, but import workflows still need an explicit gate that can require the operator to present the exact verification receipt they reviewed. Without a gate, stale verification receipts or changed expected bindings can be missed until import.

## Proposed Change

Add `molten node live-workflow-bundle-gate` and canonical `node-control-live-workflow-bundle-gate-receipt-v1` receipts. The gate re-runs bundle verification for the current expected bindings, optionally requires a supplied verify receipt, denies when the supplied receipt is missing, malformed, stale, or bound to different expectations, and prints deterministic next-step guidance for import, re-verification, or malformed bundles.

## Non-Goals

- Gate receipts do not import bundle members.
- Gate receipts are not authority, provenance, policy/resource evidence, delivery-idempotency evidence, or receiver-side ingress evidence.
- Bundle import remains fail-closed and repeats binding checks even after a passing gate.
