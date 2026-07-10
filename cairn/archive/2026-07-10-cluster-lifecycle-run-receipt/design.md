# Design: cluster-lifecycle-run-receipt

## Overview

Introduce a canonical cluster lifecycle run receipt that summarizes an entire cluster wrapper pass. The receipt should be produced by a pure core from explicit observed inputs gathered by the CLI shell.

## Functional core and shell boundary

The pure core accepts an in-memory lifecycle summary: cluster manifest ref, ordered node ids, per-node receipt refs, command decisions, already-running observations, stop ordering, diagnostics, and caveats. It validates required fields and returns a canonical Preserves value plus content ref.

The CLI/test shell owns state roots, file discovery, child command execution, stdout/stderr capture, and writing the receipt artifact.

## Receipt fields

The receipt should bind:

- cluster manifest ref and ordered node ids;
- command phases executed and their decisions;
- per-node config, identity, startup, health, queue/control, heartbeat, shutdown, and stop-control refs when required by the phase;
- already-running status refs without rewriting startup evidence;
- reverse stop order evidence;
- diagnostics and evidence-only caveats.

## Negative model

Missing phase receipts, stale manifest refs, duplicate node summaries, node-order mismatch, stdout-only evidence, and failed canonical parsing deny before pass evidence is emitted.

## Boundaries

The receipt summarizes local cluster-wrapper behavior. It is not VM evidence, live transport evidence, consensus evidence, authority evidence, or production-readiness evidence.
