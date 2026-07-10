# Design: cluster-lifecycle-summary-drift

## Overview

Define a drift-ready summary for cluster lifecycle runs. The summary should be stable across fresh roots with the same declared inputs after non-semantic variance is explicitly declared.

## Functional core and shell boundary

The pure summary core accepts a cluster lifecycle receipt or equivalent in-memory lifecycle observations and returns ordered drift fields. It does not read state roots or inspect files.

The shell runs the lifecycle twice in fresh roots, collects receipt refs, declares allowed variance for temporary roots and diagnostic logs, and invokes the existing drift comparator.

## Summary fields

Fields should include:

- workflow id and declared node ids;
- manifest ref;
- per-node config, identity, startup, status/control, heartbeat, shutdown, and stop-control refs;
- command decisions;
- already-running observations;
- stop order;
- variance refs and caveats.

## Negative model

Changed child refs, node-order changes, missing fields, field-kind drift, undeclared volatile paths, ambient state, retry-only success, and rendered-output-only success deny.

## Boundaries

The drift gate proves stability for declared deterministic inputs only. It does not retry away failures and does not convert live timing observations into deterministic evidence.
