# Design: vm-child-receipt-semantic-validation

## Overview

Add a semantic child-validation layer to the pure VM evidence validator. The CLI shell reads child artifacts from paths and passes parsed Preserves values into the core. The core classifies child receipts, extracts expected fields, and emits diagnostics without reading files itself.

## Child classes

Initial semantic validators should cover:

- node-control live workflow bundle verify/gate/apply/reconcile/ack/protocol receipts;
- live transport send/receive, ingress, queue, and control dispatch receipts;
- remote dataspace delivery or service exchange receipts;
- blob-ref job receipt and coordination apply report;
- prod-soak and VM fault validation receipts.

## Expected bindings

Validation inputs should accept explicit expected node ids, peer ids, topic, operation id or operation ref, topology ref, and required child refs. Scenario fixtures may later provide these expectations.

## Diagnostics

Diagnostics should distinguish missing child, wrong class, wrong topology, wrong node, wrong peer, wrong operation, denied child, stale ref, and log-only child. A child ref that is present but semantically wrong must deny before pass evidence is accepted.

## Boundaries

The child validator validates evidence linkage and claim scope only. It does not make child receipts authoritative for authority, policy, resource, provenance, source-gate, or retention decisions.
