# Design: local-multiprocess-cluster-tier

## Overview

Add a first-class local multiprocess tier for cluster harness workflows. It should execute real child processes while preserving deterministic planning, explicit cleanup, and evidence-only boundaries.

## Functional core and shell boundary

The pure core validates a local multiprocess cluster plan: node ids, state-root handles, transport handles, command-plan ref, expected receipt refs, ticket status, timeout policy, cleanup policy, and caveats.

The shell owns process spawning, signal handling, filesystem roots, log capture, child cleanup, timeout enforcement, and receipt collection.

## Run receipt

`local-multiprocess-executable-run-v1` or a cluster-specific extension should bind:

- plan ref and fixture ref;
- startup, workflow, shutdown, and cleanup refs;
- child timeout and orphan observations;
- ticket freshness;
- cleanup success;
- diagnostics and local-evidence caveats.

## Negative model

Plans deny state-root collisions and transport collisions before process launch. Runs deny stale tickets, child timeouts, orphaned processes, missing receipts, and cleanup failures before pass evidence.

## Boundaries

This tier is local process integration evidence, not NixOS VM, remote network, WAN, production, or authority evidence.
