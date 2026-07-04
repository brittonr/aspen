# Design: vm-scenario-gate-integration

## Overview

The functional core remains the existing fixture, topology, reconciliation, and live-transport validation logic. The VM shell supplies only already-materialized VM artifacts: topology, node evidence, child workflow receipts, diagnostics, and the checked scenario fixture export. A VM gate builder derives canonical gate receipts from those explicit inputs.

## Receipt flow

1. Validate the Nickel scenario fixture export and emit `multinode-scenario-metadata-v1`.
2. Validate topology membership against the selected topology profile and emit `multinode-topology-membership-gate-v1`.
3. Build per-node summaries from VM node evidence and child receipts.
4. Run `multinode-reconciliation-gate-v1` for queue, ledger, dispatch, ack, protocol, and child receipt equality classes.
5. For live-control shards, run `nixos-vm-live-transport-gate-v1` over ticket, peer admission, authority, send, receive, ingress, queue, dispatch, reconcile, ack, and protocol-gate refs.
6. Add gate refs to shard and aggregate manifests.

## Negative gates

Negative fixtures mutate one input at a time: wrong scenario fixture ref, wrong topology profile, missing receive receipt, divergent queue ref without variance, stale protocol gate, duplicate semantic commit, or diagnostic log substituted for a receipt.

## Boundaries

The gates validate VM evidence shape and reconciliation only. They do not grant authority, policy, provenance, resource, source-gate, retention, transport trust outside the VM topology, or production-readiness claims.
