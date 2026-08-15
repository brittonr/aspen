# Design: vm-evidence-child-manifest

## Overview

Strengthen the VM evidence manifest from a top-level artifact index into a closure over all canonical child receipts referenced by the run. The CLI shell continues to walk explicit paths supplied by the VM script; the pure validator checks manifest entry consistency over in-memory values.

## Manifest content

Manifest entries should include topology, node evidence, VM test-run, prod-soak, validation, shard receipts, aggregate receipts, live-control child receipts, service/job/coordination receipts, fault descriptors and receipts, fault validation, support matrices, and diagnostic logs.

## Closure checks

The closure validator should report:

- referenced child ref missing from manifest;
- manifest entry file missing or unreadable;
- content ref mismatch;
- duplicate path or duplicate semantic artifact;
- unsupported or wrong artifact kind;
- required child omitted;
- log-only child represented as canonical evidence;
- unreferenced required evidence in a shard output.

## Boundaries

The manifest is review and preservation evidence. It does not make a child receipt authoritative outside its own subsystem gate and does not let logs repair missing canonical artifacts.
