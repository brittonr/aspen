## Context

`runtime-service-core` already defines portable service specs, instances, routes, receipts, and a Forge slice. The missing seam is a canonical operator contract that connects runtime-host loading evidence with jobs/plugins/deploy operations without claiming a full scheduler exists.

## Goals / Non-Goals

**Goals:** define typed boundaries and receipts that future implementation can drain in small slices.

**Non-Goals:** implement a scheduler, migrate all services, or replace existing job/plugin/deploy internals immediately.

## Decisions

### 1. Contract before orchestration rewrite

**Choice:** Specify the canonical contract as a model/adapter layer before moving execution code.

**Rationale:** It avoids accidental broad rewrites and preserves existing product paths.

### 2. State-boundary vocabulary

**Choice:** Distinguish validated, admitted, scheduled, started, healthy, failed, and stopped states.

**Rationale:** Runtime-host proof and service-readiness claims are otherwise easy to conflate.

## Risks / Trade-offs

**Too abstract** → Require concrete receipt identities and adapter touchpoints.

**Too broad** → Leave implementation tasks open and drain one service/backend slice at a time.
