## Why

Upgrade drains are state machines over session evidence, task completion, compatibility refs, rewrite/apply evidence, and cutover. A drain must not complete or cut over until terminal protocol lifecycle evidence is present for the affected protocol and stale or wrong-protocol gates deny before mutation.

## What Changes

- Add requirements for upgrade drain state proof.
- Require proof traces that bind task refs, from/to protocol refs, protocol session gate receipts, terminal state refs, and cutover receipts.
- Require negative evidence for missing gate, denied gate, wrong protocol, stale compatibility ref, missing terminal state, and no-mutation denial.

## Impact

- **Files**: upgrade session drain logic, protocol gate integration, structured rewrite upgrade hooks, and tests.
- **Testing**: passing drain with terminal gate, missing/wrong/stale protocol gate denial, denied terminal evidence, and cutover side effects absent on denial.
