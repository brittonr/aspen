## Why

Molten documents semantic Nextest profiles such as `fast-core`, `harness`, `cli`, `distributed-simulation`, `vm-platform`, and `dogfood-soak`, but the current `.config/nextest.toml` mostly gives those profiles timeout/JUnit differences. Without explicit partitions, a profile can accidentally include tests outside its evidence claim, such as live/VM tests in deterministic evidence or diagnostic-only tests in release evidence.

The test configuration should make each profile's evidence scope machine-checkable and should fail closed when a deterministic or release profile includes suites that are non-replayable, platform-only, or diagnostic-only without an explicit exclusion.

## What Changes

- Add a reviewed profile manifest or Nextest configuration convention that binds each profile to filters, expected artifacts, retry policy, evidence scope, and platform availability.
- Partition `fast-core`, `harness`, `cli`, `distributed-simulation`, `vm-platform`, and `dogfood-soak` so profile names match the tests they run.
- Keep exploratory retries visibly diagnostic and excluded from deterministic pass evidence.
- Extend the existing `nextest-config` check/readback so it validates profile filters, JUnit paths, retry behavior, and non-replayable exclusions.
- Add positive and negative tests or fixtures for profile partitioning, missing filters, deterministic/live mixing, and JUnit-only evidence misuse.

## Impact

- **Files**: `.config/nextest.toml`, test naming/metadata conventions, Nix `nextest-config` check, testing hardening helpers, README/proof workflow docs.
- **Testing**: positive profile readback and negative profile-mixing fixtures.
- **Safety**: Nextest profile receipts remain test evidence only and do not replace canonical subsystem receipts, source gates, policy, provenance, authority, transport, resource, retention, release, or execution gates.
