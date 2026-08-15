## Why

Syndicate/SAM dataspaces are Molten's normal actor interaction model, but some operations require strongly consistent coordination: locks, leases, queues, semaphores, rate limits, leader election, barriers, and fencing tokens. Aspen's CAS/Raft-backed coordination primitives are useful prior art. Molten should provide these as control-plane services exposed through dataspace assertions, not as the default actor mailbox or message substrate.

## What Changes

- Define policy-gated coordination primitives backed by strongly consistent control-plane state.
- Include the admitted initial primitive set: locks with fencing tokens, lease/election-style grants, explicit FIFO queues, semaphores, rate limiters, barriers, and service registry entries. Rich queue visibility timeout/ack/nack/DLQ and counter/sequence services remain future extensions until admitted separately.
- Expose coordination requests/results as dataspace assertions and effect receipts.
- Use Trellis/Raft/control-plane storage for linearizable state where required.
- Keep ordinary actor messages, assertions, choreography steps, and job traffic off Raft unless explicitly using a coordination primitive.

## Impact

Molten gets practical distributed coordination without compromising the SAM actor model. The completed milestone implements deterministic local/control-plane service semantics and the Raft-backed control-plane contract for locks, fencing tokens, explicit queues, semaphores, rate limits, elections, barriers, and service registry records.
