## Why

Fast CLI tests and NixOS VM tests cover different risks, but there is room for a middle tier that runs real local processes with isolated state roots and transport handles. That tier can catch child timeouts, stale tickets, orphaned processes, cleanup failures, and state-root collisions before expensive VM checks.

## What Changes

- Promote the existing local multiprocess model into a cluster harness tier with fixture-derived plans and canonical executable-run receipts.
- Run cluster workflows through child processes with isolated state roots, declared transport handles, timeout policy, cleanup policy, and expected receipts.
- Add negative fixtures for state-root collisions, transport collisions, stale tickets, child timeout, orphaned children, missing workflow receipts, and cleanup failure.
- Keep shell observations explicitly local evidence and not VM or live WAN evidence.

## Impact

Developers get faster integration feedback and clearer failure artifacts before VM checks. Local multiprocess evidence remains local integration evidence only and does not grant authority, policy, provenance, source-gate, resource, transport, retention, deployment, or production trust.
