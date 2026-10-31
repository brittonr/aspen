# Implementation review: host admitted recovery groups in the public node

## Disposition

The public node hosts admitted recovery groups through capability-rooted durable state, the real
native supervisor lifecycle and exact current-authority operations, and the real-process twin
passed with correct and three broken cohorts. This is implementation review, not whole-system
correctness or release authority. Evidence: `evidence/2026-10-31-live-recovery-twin.log`.

## Reviewed invariants

- Group state, restart history, fences, work claims and effect intents are durable and
  capability-rooted; planners stay pure and add no effects.
- Public operations require exact current authority; queue acceptance is not a commit.
- Reopening observes the owners' journals; uncertain effects are not retried implicitly.
- The twin constructs no test-owned supervisor: it runs the independently source-admitted node,
  kills the serving process and reopens real journals.

## Defect found and fixed

Each public command reloads native supervisors from their journals, and a never-started instance
is journaled in phase `Installed`. Starting it applied `Install` again and was refused with
`IllegalTransition { phase: Installed, event: Install }`. `Dispatcher::activate` now resumes a
recovered `Installed` host at admission and still refuses every other phase; regression test
`recovered_installed_host_activates_from_admission_and_running_host_cannot_reactivate`.

## Non-claims

Single-host evidence only. No VM, multi-host, Byzantine, production or exactly-once claim.
