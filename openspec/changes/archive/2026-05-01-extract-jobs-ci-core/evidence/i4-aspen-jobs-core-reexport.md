# I4 aspen-jobs compatibility re-export evidence

## Scope

`aspen-jobs` now depends on `aspen-jobs-core` and re-exports the portable core surface through a namespaced compatibility module while preserving the historical runtime crate root exports.

Implemented surface:

- `aspen_jobs::core::*` re-exports all portable contracts from `aspen-jobs-core`.
- `aspen_jobs::CoreJobEvent` aliases `aspen_jobs_core::JobEvent`.
- `aspen_jobs::JobsCoreError` aliases `aspen_jobs_core::JobsCoreError`.
- `aspen_jobs::transition_core_status` aliases `aspen_jobs_core::transition_status`.

The existing runtime compatibility shell exports such as `aspen_jobs::JobSpec`, `aspen_jobs::JobStatus`, `aspen_jobs::RetryPolicy`, `aspen_jobs::Schedule`, and worker/manager APIs remain unchanged at the crate root for existing consumers.

## Command

```text
cargo check -p aspen-jobs
```

## Result

```text
    Checking aspen-jobs-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-jobs-core)
    Checking aspen-jobs v0.1.0 (/home/brittonr/git/aspen/crates/aspen-jobs)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 22.10s
```

Non-fatal workspace warnings were emitted for vendored `iroh-h3` resolver configuration, matching the known baseline.
