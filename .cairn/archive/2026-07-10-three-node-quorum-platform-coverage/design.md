# Design: three-node-quorum-platform-coverage

## Overview

Add an executable three-node VM shard as a platform shell around the existing pure topology, membership, quorum, and reconciliation gates. The shard should produce node summaries and feed them into `multinode-reconciliation-gate-v1` and quorum-specific receipts.

## Scenario shape

The shard uses three VM nodes with explicit roles:

- two or three voting members for majority evidence;
- one restarting member for restart/rejoin evidence;
- optional subscriber or observer evidence used only for negative membership checks.

## Evidence requirements

Passing evidence must bind topology profile refs, membership refs, quorum refs, per-node summaries, queue/ledger/dispatch/ack/protocol refs, duplicate-suppression evidence, restart/rejoin evidence, reconciliation receipt, and caveats.

Negative evidence must show that subscriber, observer, transport-only, partitioned-minority, missing-quorum, and log-only evidence cannot satisfy voter membership or authority.

## Boundaries

Three-node VM evidence is bounded platform integration evidence, not a fleet-scale consensus proof or WAN reliability claim. The test must not promote transport observations into authority, policy, or membership evidence.
