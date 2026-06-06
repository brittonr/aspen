# Change: Node Control Live Import UX

## Motivation

Live node-control workflows now have tickets, peer admissions, authority grants, send receipts, and runbook receipts, but multi-state-root operator workflows still require callers to manually preserve and hash remote ticket/admission/grant artifacts. Molten needs explicit import commands that validate these artifacts before making them available to a local node ledger, while preserving the rule that import/transport evidence is not authority or provenance by itself.

## Proposed Change

Add a live import UX for node control:

- `molten node live-ticket-import` imports a receiver live ticket and optionally its peer-admission receipt into a local state root after validating node/topic/endpoint/peer binding and admission freshness.
- `molten node authority-grant-import` imports a node-control authority grant into a local state root after validating peer/node/operation/scope/epoch/revocation binding.
- Both commands emit canonical import receipts that record pass/deny diagnostics and imported refs.
- Live-send diagnostics point operators toward importing bound live tickets when offline tickets lack endpoint address evidence.

## Non-Goals

- Import receipts do not satisfy peer bootstrap, operation authority, policy/resource, delivery-idempotency, or payload provenance gates.
- This change does not replace receiver-side ingress validation; receivers still resolve admitted peer admissions and authority grants from their own state roots before enqueue.
- This change does not make live transport runs deterministic replay evidence.
