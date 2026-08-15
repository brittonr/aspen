# Design: executable-three-node-quorum-vm

## Overview

Wire the existing three-node quorum fixture and pure gates to an executable VM shard. The shard should prove only that the bounded VM topology produced the expected child receipts for quorum-shaped behavior.

## Functional core and shell boundary

The pure core validates topology membership, quorum evidence, reconciliation evidence, duplicate semantic commit suppression, and scenario fixture alignment from in-memory refs.

The VM shell owns NixOS VM creation, systemd service control, network setup, command execution, and artifact collection.

## Scenario coverage

Executable coverage should include:

- three explicit nodes with voter/member roles;
- majority commit receipt accepted;
- minority or partitioned quorum denied;
- restarting member rejoins and produces recovery evidence;
- subscriber or observer cannot satisfy voter membership;
- duplicate semantic commits are suppressed or denied before a second commit is accepted.

## Aggregation

The shard emits a scoped receipt and feeds VM scenario, reconciliation, aggregate, and failure-bundle gates. Aggregates must retain the bounded-topology caveat.

## Boundaries

The shard is a platform integration check for a small VM topology. It is not fleet-scale, WAN, or whole-system consensus proof.
