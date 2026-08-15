# Design: three-node-vm-quorum-topology

## Overview

Add a small three-node topology profile to the existing multinode VM evidence surface. The functional core declares the role membership and expected receipt classes; the VM shell starts the extra node only for the targeted shard or profile.

## Topology shape

The profile includes:

- two voter/control members that can form majority evidence with the restarting member;
- one restarting member used for restart/rejoin and duplicate suppression checks;
- optional subscriber or observer role for negative membership checks when the scenario requires it.

## Scenarios

- Majority path: admitted voters produce matching queue, ledger, dispatch, ack, and protocol refs.
- Minority partition: a partitioned minority cannot satisfy quorum pass evidence.
- Restart/rejoin: a restarting member rejoins without producing duplicate semantic commits for the same operation id.
- Subscriber confusion: subscriber or transport-only evidence is rejected when used as voter membership or authority evidence.

## Evidence

Three-node VM receipts bind topology profile, node roles, membership refs, quorum refs, per-node summaries, reconciliation gate refs, and caveats. Any partition, restart, or membership denial emits canonical diagnostics before pass evidence is accepted.

## Boundaries

This is a minimal platform integration topology, not fleet-scale performance or WAN evidence. Logs remain diagnostic-only and cannot replace quorum, membership, or reconciliation receipts.
