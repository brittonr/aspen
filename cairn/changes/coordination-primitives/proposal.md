## Why

Syndicate/SAM dataspaces are Molten's normal actor interaction model, but some operations require strongly consistent coordination: locks, leases, queues, semaphores, rate limits, leader election, barriers, and fencing tokens. Aspen's CAS/Raft-backed coordination primitives are useful prior art. Molten should provide these as control-plane services exposed through dataspace assertions, not as the default actor mailbox or message substrate.

## What Changes

- Define policy-gated coordination primitives backed by strongly consistent control-plane state.
- Include locks with fencing tokens, leases/elections, queues with visibility timeout and DLQ, semaphores, rate limiters, counters/sequences, barriers, and service registry entries.
- Expose coordination requests/results as dataspace assertions and effect receipts.
- Use Trellis/Raft/control-plane storage for linearizable state where required.
- Keep ordinary actor messages, assertions, choreography steps, and job traffic off Raft unless explicitly using a coordination primitive.

## Impact

Molten gets practical distributed coordination without compromising the SAM actor model. The first milestone can implement local/mock service semantics and specify the Raft-backed control-plane contract for locks, fencing tokens, and queues.
