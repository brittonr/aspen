# Change-local acceptance

a[octet-burndown-safety-clock.zero] Pinned Octet root and `-p molten --lib` summaries report 0 `ambient_clock` and 0 `no_recursion`, and no other lint family count increases.
a[octet-burndown-safety-clock.allows] The only allows added are the two `tigerstyle::ambient_clock` item allows on `LiveClockAdapter::new` and `LiveClockAdapter::observe_wall`, each with a reason naming the live clock capability.
a[octet-burndown-safety-clock.bounds] `TickDeadline` expires exactly at its timeout and denies overflow; `SupervisionDeadline` admits its one-hour bound and denies one nanosecond past it; the structural scan admits its exact depth and node bounds and denies one past.
a[octet-burndown-safety-clock.behavior] Supervision timeouts and structural-scan results are unchanged; fmt, clippy with `-D warnings`, focused tests, and the workspace tests pass.
