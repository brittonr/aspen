# Proposal: Bind scheduler completions to the lease epoch

## Why

The reference distributed scheduler stores `lease_epoch` and increments it on `Lease` and `Failover`, but `DistributedSchedulerOperation::Complete` carries only the job, owner, and completion reference. The transition checks the current owner only.

A worker can hold a stale completion from an earlier assignment and have it accepted after ownership returns to it:

```text
Submit job J            epoch = 0
Lease J to worker A     epoch = 1
Failover J to worker B  epoch = 2
Failover J to worker A  epoch = 3
A's epoch-1 completion arrives; the owner matches, so it is accepted.
```

The coordination-delivery service already specifies a stronger binding: its token covers the queue, item, consumer, attempt, cycle, fencing token, consistency epoch, service generation, and policy, and every ack, nack, and extension requires the current token. The reference scheduler should meet the same standard.

The observation path also cannot catch this defect today. Semantic invariant results are copied from the transition's own reported invariant names, and safety observation flags are hard-coded to their passing values. A stale completion accepted by the service therefore passes the fixture.

## What Changes

- Add an exact assignment token to the scheduler completion request, derived from the job, the owner, and the current lease epoch at lease time.
- Reject a completion whose token does not match the current assignment, even when the owner matches.
- Add a permanent regression for the owner-returns trace: lease A, fail over to B, fail over back to A, replay A's first-assignment completion, and require rejection.
- Add an independent stale-completion check over the transition history so the checker detects an accepted stale completion even when a defective service accepts it.
- Keep coordination-delivery token semantics unchanged; reuse them as the design reference.

## Impact

- **Files**: `molten-core` reference scheduler state machine and operations, reference operation fixtures, simulation composition invariant evaluation inputs, and related tests.
- **Testing**: owner-returns regression, wrong-token rejection, token replay after completion, duplicate-completion behavior, epoch overflow, and an independent checker negative test against a deliberately broken transition.
- **Non-goals**: no change to the live coordination-delivery service, no new scheduler features, and no claim that other delivery paths share this defect.

## Dependencies

- `molten-core` reference services and fabric simulation contracts.
- `add-protocol-aware-simulation-oracles` provides the independent-evaluation direction; this change stays implementable without it.
