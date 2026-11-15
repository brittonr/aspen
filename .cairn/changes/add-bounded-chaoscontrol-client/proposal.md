## Why

Aspen #40 and ChaosControl #19 require retained real client operations, rather than expected-array comparisons. Native multi-process Raft already supplies admitted execution; guest cohort execution remains ChaosControl-owned.

## What Changes

- Define a canonical, finite versioned native Raft scenario with explicit participant, request, payload, phase and recovery bounds.
- Execute through the existing live-process adapters using WorkloadExecutor and independently check actual application, quorum and recovery readbacks.
- Keep fault intent separate from application and retain participant-scoped fault-off horizons.
- Add a deployed and reached duplicate-application semantic control with property suppression and valid-prefix reduction.
- Document the existing guest intake dependency handoff and unavailable guest/cohort dependencies without qualifying a native test as a guest.

## Impact

Owned surfaces are fabric_simulation/bounded_client, Raft live_process, dedicated fixtures and documentation. No executor, transport runtime, fault engine, root dependency or qualification report is introduced.
