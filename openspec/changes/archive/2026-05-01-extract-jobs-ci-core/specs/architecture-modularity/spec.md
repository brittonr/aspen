## ADDED Requirements

### Requirement: Extraction inventory tracks jobs/CI core boundary [r[architecture-modularity.extraction-inventory-tracks-jobs-ci-core]]
The crate extraction inventory SHALL track the jobs/CI core boundary separately from jobs/CI runtime shells so reusable scheduler/config/run-state APIs can advance independently from workers, handlers, executors, and app integrations.

#### Scenario: Inventory row identifies core boundary [r[architecture-modularity.extraction-inventory-tracks-jobs-ci-core.inventory-row-identifies-core-boundary]]
- GIVEN the jobs/CI extraction family is updated
- WHEN `docs/crate-extraction.md` and `docs/crate-extraction/jobs-ci-core.md` are read
- THEN the inventory SHALL identify the reusable core crate(s), compatibility shell crate(s), runtime adapter crate(s), owner status, readiness state, and next action
- AND policy/checker evidence SHALL prove reusable defaults avoid forbidden app, handler, transport, worker, executor, storage, process, VM, Nix, and SNIX dependencies unless those dependencies are explicitly part of a runtime adapter surface
