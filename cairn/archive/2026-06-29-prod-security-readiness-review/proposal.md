## Why

Molten has explicit authority, provenance, redaction, retention, and source-gate evidence. Before production, those mechanisms need a security-readiness pass that exercises the threat model and failure drills rather than only proving happy-path schema behavior.

Production risk concentrates around keys, delegation/revocation, sensitive artifact admission, secret/redaction paths, live transport trust boundaries, plugin/Wasmtime/Steel hostcalls, and supply-chain evidence. These need reviewable receipts and negative tests before customer-critical workloads are admitted.

## What Changes

- Add a production threat model that maps assets, principals, trust boundaries, and attack scenarios to Molten gates.
- Add key, delegation, and revocation drills with canonical receipts.
- Add boundary fuzzing/negative suites for Preserves parsers, receipt validators, node-control ingress, repro bundles, plugin hostcalls, and provenance inputs.
- Add a secrets/redaction audit over logs, summaries, exports, catalogs, MCP responses, repro bundles, and failure diagnostics.
- Add supply-chain/provenance review evidence for release artifacts and sensitive remote installs.
- Add an incident-response drill for compromised key, leaked ticket, stale source gate, and bad release evidence.

## Impact

This creates the security checklist for moving from internal pilot to broader production. Passing security-readiness evidence is still review evidence; it does not grant authority or bypass runtime subsystem gates.
