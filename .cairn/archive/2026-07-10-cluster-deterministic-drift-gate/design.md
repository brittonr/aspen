# Design: cluster-deterministic-drift-gate

## Overview

Create a drift gate over cluster lifecycle evidence. The shell runs workflows in fresh roots; the pure comparator receives evidence summaries, canonical refs, normalized values, and allowed variance declarations.

## Workflows

Initial workflows:

- cluster `init → start → status → stop` over two nodes;
- already-running `start` path;
- denied malformed-manifest path;
- selected VM child evidence summaries where deterministic replay is claimed;
- manifest closure validation over stable artifacts.

## Variance model

Allowed variance must be explicit and canonical. Runtime paths, temporary roots, store paths, diagnostic logs, and rendered output may be normalized only when declared. Operation refs, receipt refs, node ids, topology refs, manifest refs, and semantic decisions should match unless a specific variance ref explains otherwise.

## Negative fixtures

Negative fixtures should inject changed child refs, undeclared volatile fields, ambient state, map-order instability, retry-only success, and rendered-output-only success.

## Boundaries

Drift comparison is not a retry loop. A pass after retry is not deterministic pass evidence. Live-only observations remain non-replayable unless recorded effect logs are included as declared inputs.
