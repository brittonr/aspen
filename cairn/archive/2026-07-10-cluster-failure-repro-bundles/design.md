# Design: cluster-failure-repro-bundles

## Overview

Create sealed diagnostic repro bundles for cluster-related failures. The bundle packages enough canonical refs to review a failure while preserving redaction and non-pass boundaries.

## Functional core and shell boundary

The pure core builds and verifies bundle payload values from explicit refs: scenario fixture, topology, scheduler or command plan, seed or effect log, node summaries, child receipts, diagnostics, logs, redaction policy, replay status, private attachments, reveal receipts, and caveats.

The shell owns artifact discovery, bundle directory creation, private attachment handling, redaction transform files, and optional unpack materialization.

## Bundle cases

Initial cases:

- cluster lifecycle denial;
- local multiprocess child timeout or cleanup failure;
- VM unavailable host support;
- VM fault validation denial;
- reconciliation or drift gate denial.

## Verification rules

Verification recomputes payload refs, rejects tampering, checks redaction policy refs, rejects private attachments without reveal receipts, and marks non-replayable VM/local observations as diagnostic-only.

## Boundaries

Failure bundles help review failures. They cannot satisfy pass gates unless a future explicit policy defines and accepts a gate-preserving transform.
