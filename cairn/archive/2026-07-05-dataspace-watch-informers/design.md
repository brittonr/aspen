# Design: dataspace watch informers

## Context

Molten's dataspace can route assertions and retractions. This change layers a resource watch/informer contract on top of that model for declarative resources. It borrows the list/watch/informer idea from Kubernetes but keeps Molten's canonical Preserves, capability, and replay boundaries.

## Watch model

A watch event should bind:

- resource ref, type, scope, and generation;
- event kind such as added, modified, deleted, bookmark, or compacted;
- revision cursor before and after the event;
- admission receipt refs for the change;
- selector and observer authority refs;
- event body ref and evidence refs.

The cursor is a replay boundary, not a global wall-clock timestamp.

## Informer model

An informer snapshot is valid only when it binds an initial list ref, a starting cursor, the sequence of watch events applied after the list, a final cursor, selector refs, and observer authority evidence. Resync should produce receipts that prove whether the cache remained current, resumed from a cursor, or was forced to relist after compaction.

## Functional core

Pure cores validate selector bounds, cursor transitions, event ordering, cache state transitions, and stale-cursor diagnostics over in-memory event summaries. The shell owns store reads, dataspace subscription setup, persistence, and network transport.

## Authority boundary

Selectors are capabilities. A broad selector or cross-scope watch must carry authority evidence. Unauthorized selector expansion denies before current resources are revealed.
