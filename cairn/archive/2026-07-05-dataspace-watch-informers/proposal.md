# Change: dataspace-watch-informers

## Why

Kubernetes controllers work well because they can list current objects, watch ordered changes, and maintain informer caches. Molten already has dataspace observations, but resource-shaped controllers need a canonical watch/cursor/informer contract so they can reconcile from evidence instead of polling stores or scraping logs.

## What

- Add resource watch streams over dataspace-backed assertions with canonical revision cursors and event refs.
- Add informer snapshot rules that combine an initial list with subsequent watch events without missing or reordering resource generations.
- Gate selector use through capability and policy checks so watches cannot become ambient discovery.
- Add compaction/stale-cursor denial diagnostics and replay/resume receipts.

## Impact

Controllers can react to resource changes deterministically and recover after interruption. Watch streams remain Molten dataspace/evidence artifacts, not Kubernetes watch API compatibility.
