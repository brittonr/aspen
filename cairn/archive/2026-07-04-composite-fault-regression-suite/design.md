# Design: composite-fault-regression-suite

## Overview

Build on existing deterministic simulation and generated-case repro artifacts by adding a curated composite regression suite. The suite is small enough for protocol CI, deterministic enough for review, and explicit about which generated failures were promoted.

## Composite cases

Initial named cases cover:

- duplicate delivery after sender or receiver restart;
- partition while stale evidence is presented;
- message reorder while ack/reconcile evidence is checked;
- crash or stop during queue-to-dispatch transition;
- resource pressure while a quorum-requiring command is in flight;
- ambient-state drift or unauthorized transport evidence paired with another benign fault.

## Promotion workflow

A generated case can be promoted only when it has a stable seed, topology ref, scheduler ref, fault-plan ref, command refs, invariant name, failure diagnostics, replay ref, and expected pass or deny decision. Promotion writes a named fixture and adds traceability coverage for positive and negative evidence.

## Budget and CI metadata

Each composite case declares cost class, profile eligibility, expected artifact kinds, variance refs, diagnostic logs, and release-review status. Protocol CI should run the compact suite. VM and soak profiles may bind representative child refs but must not claim generated simulation pass evidence as VM or production readiness.

## Boundaries

Retry success is diagnostic-only. Composite simulation evidence does not grant authority, policy, provenance, resource, source-gate, retention, VM platform, live WAN, or production claims.
