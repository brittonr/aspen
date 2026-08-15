## Goals

- Record a durable-store-coordinated ownership option.
- Require ownership transfer to be a single CAS lease in this mode.
- Require replaceable nodes and no fixed membership list.
- Leave runtime transport and evidence authority with Aspen and its consumers.

## Ownership contract

In the durable-store-coordinated mode, ownership of an entity is a compare-and-swap lease in a durable dataspace.

The core accepts these values:

1. the current lease owner;
2. the claimed new owner;
3. the current epoch;
4. the claimed new epoch.

The core returns an ownership decision from these supplied values. It reads no store, clock, or network.

## Decision rules

The pure core returns one of two decisions:

- `Acquire` — the claimed owner matches the current owner and the epoch advances.
- `Reject` — the lease does not match, and ownership does not change.

A node that loses a lease cannot damage the state. A spare acquires a released lease only when normal traffic reaches it.

## Output

The decision contains public, bounded fields:

- the owner and epoch before and after;
- the disposition;
- implementation and contract identities;
- required non-claims.

The decision does not prove runtime correctness, data integrity, or release readiness.

## Functional core and shell

The pure core computes the lease decision from supplied values.

The shell and consumers own the durable dataspace, transport, and evidence. This mode is a design option, not a replacement of current behavior.

## Reference

The Celld ownership model (CAS lease, replaceable nodes, bucket as source of truth) is a bounded reference input. It is a comparison source, not an Aspen requirement, parity claim, or equivalence claim.

## Verification

Positive coverage includes a matching-owner acquisition with an advanced epoch.

Negative coverage includes a mismatched owner, a stale epoch, and a node that lost its lease.

Boundary coverage rejects store, clock, or network reads, fixed-membership coupling, and correctness or integrity claims.
