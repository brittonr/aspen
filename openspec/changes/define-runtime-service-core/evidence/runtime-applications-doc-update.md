# Runtime applications documentation update

- Change: `define-runtime-service-core`
- Task: update `docs/runtime-applications.md`
- Started: `2026-05-07T00:30:08Z`
- Completed: `2026-05-07T00:30:57Z`

## Updated

`docs/runtime-applications.md` now distinguishes implemented runtime-service-core pieces from future runtime shell work:

- documents `crates/aspen-runtime-core` as the current portable model crate;
- documents linked `NativeBuiltIn` services and the absence of dynamic native plugin loading;
- documents the Forge runtime wrapper in `crates/aspen-forge/src/runtime_service.rs`;
- marks durable reconcilers, route-registry integration, node-local host supervision, dynamic app install, and executioner generalization as follow-up work;
- rewrites the “First implementation slice” section to separate completed pieces from remaining migration-track work.

## Verification

Documentation-only update reviewed with `git diff --check` as part of final validation.
