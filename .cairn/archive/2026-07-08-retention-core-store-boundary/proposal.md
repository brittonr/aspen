## Why

Retention combines admission, destructive-operation safety, GC plan/apply/audit chains, bundle export, remote clearance, live workflow, and local store IO. These concerns need sharper boundaries so deletion-safety decisions remain pure and reviewable while shells own persistence and transport.

## What Changes

- Separate retention admission core, GC planning core, retention store adapter, bundle/export shell, and live-clearance transport shell.
- Ensure destructive decisions return explicit plans before any delete, tombstone, redaction, or remote-clearance import occurs.
- Add positive and negative tests for admitted and denied retention plans.

## Impact

Retention remains fail-closed while becoming easier to test and extract. Destructive side effects become visibly downstream of deterministic admission decisions.
