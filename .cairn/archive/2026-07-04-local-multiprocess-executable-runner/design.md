# Design: local-multiprocess-executable-runner

## Overview

Use the existing local multiprocess plan as the functional core. Add a thin imperative shell that validates the plan, creates temporary state roots, spawns `molten node` processes, drives a small control workflow, collects canonical receipts, and tears down every process and state root according to the declared cleanup policy.

## Runner phases

1. validate plan: reject colliding state roots, colliding transport handles, stale tickets, missing expected receipt refs, and missing cleanup policy;
2. prepare roots: create isolated state roots and write setup receipts;
3. start nodes: run `molten node init` and `molten node run` or `serve` with explicit receipt outputs;
4. execute workflow: submit a bounded control request across local transport handles;
5. collect evidence: gather startup, workflow, shutdown, cleanup, reconciliation, and diagnostic refs;
6. cleanup: terminate children, remove or preserve roots according to policy, and emit cleanup receipts.

## Failure handling

Any child exit, timeout, missing receipt, stale ticket, orphaned process, or cleanup failure records denial evidence. Pass evidence requires all required startup, workflow, shutdown, and cleanup refs.

## Boundaries

The runner is local integration evidence only. It does not satisfy NixOS VM, systemd, QEMU networking, executable VM fault, live WAN, deployment, or production-readiness claims.
