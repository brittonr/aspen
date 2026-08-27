# Bounded content replication

Molten implements content replication as an optional system extension. Ordinary content reads and writes do not activate replication.

## Ownership

The extension owns replica policy, placement, repair, handoff, and convergence status. Content stores retain byte identity, verification, transforms, protection, and local availability.

Fabric adapters retain transport, time, membership, placement, durable-state, resource, identity, and observation mechanisms. These adapters do not define replica policy.

## Pure planner

The functional core receives a complete `ReconcileInput`. This input contains the manifest, inventory, peers, operation history, and observed tick.

The manifest binds these facts:

- the service identity and generation;
- the membership and placement epochs;
- the content and transport profiles;
- the replica, repair, retention, resource, and evidence policies;
- all required fabric ports;
- the fixed non-claims.

The planner returns ordered transfer, repair, handoff, reuse, defer, and cleanup actions. It performs no file, network, clock, or process operation.

Operation identity includes the content and manifest references. It also includes source, receiver, generation, epochs, action, transform, protection mode, and attempt.

Current replicas must match the active generation and epochs. Stale replicas can supply verified bytes, but they cannot satisfy current placement.

## Imperative shell

The shell observes authority, identity, membership, placement, time, inventory, and durable history before it runs the planner.

For each transfer, the shell uses this order:

1. Acquire the retention pin.
2. Request the exact receiver-owned operation.
3. Validate the transport envelope.
4. Verify the content through the content port.
5. Store the terminal operation.
6. Publish the operation observation.

The shell stores and publishes status after all actions. It publishes the aggregate receipt last.

Cleanup requires the content rule authority and a matching cleanup clearance. An active retention pin prevents cleanup.

## Conformance

The deterministic profile uses the accepted simulated content, transport, and durable-state adapters. Fault tests cover cancellation, partition, timeout, unavailable peers, corruption, and crash-before-mutation.

The live loopback profile uses the existing Iroh transport adapter and capability-rooted local content adapter. Both profiles run the same planner and shell.

The multiprocess adapter sends exact content bytes through two child processes. The request reference is the planned operation identity.

The transport harness indexes the request and payload inputs. Offline verification binds the parent receipt, child receipts, payload, and operation request.

## Operator status

The bounded operator view reports these facts:

- desired and verified replica counts;
- the placement epoch and active plan;
- under-replicated content;
- active operations and transfer resources;
- failures and retention pins;
- evidence references and non-claims.

The view contains no content bytes, credentials, private keys, transport handles, or backend objects.

## Non-claims

A replica count does not prove permanent durability. A local repair does not prove global availability.

Transfer completion does not prove exact-once delivery. Replication does not grant installation, execution, publication, merge, or deletion authority.

Replication preserves protected content. It does not decrypt or reveal that content.
