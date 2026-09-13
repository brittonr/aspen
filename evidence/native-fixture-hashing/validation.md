# Native fixture hashing validation

## Result and ownership

The default and CI nextest commands pass all 1,646 tests after a BLAKE3-only Cargo build optimization.
The native fixtures, assertions, scenarios, exact-byte reads, runtime limits, and nextest profiles remain unchanged.
This result applies to the candidate based on `89f63d71e3f7bf45b141be0e6772470f5bb9908c`.
It does not include retry-domain commit `06837c6a619d0a21491dd0830119da123d2ece72`.

Molten owns this development configuration. Its repository maintainers own its upkeep.
The immediate consumer is the native system-extension suite, which hashes complete executable bytes during cohort setup.
The durable contribution is a package-level optimization that applies throughout workspace development/test builds.
It does not apply only to fixture setup. Release builds remain separate.

No runtime cache, alternate executable, synthetic identity, producer change, or new port is present.
Recompiled executable bytes can change their content references. The fixtures observe the exact current bytes.

Maintainers must review this override after changes to BLAKE3 features or fixture build size.
Removal requires the unchanged default and CI commands to pass without the override.

## Before-change evidence

The original source archive and input manifest bind the native fixtures and protected configuration.
Only temporary timing output changed during the diagnostic. The archive retains that exact source and diff.
The coordinator restored the original source before the candidate tests.

- Build task 1389 passes in 4m 08s under a separate 24-minute cold-build allocation.
- Default task 1393 runs the two affected cases with two test threads. Both time out near 60 seconds, exit 100.
- Each reads 927,552,568 bytes in less than one second. Neither completes its first content-reference call before the timeout.
- CI task 1425 runs the six-scenario executor case. It times out at 180.080 seconds, exit 100.
- Four completed hash calls take 39–43 seconds each. The fifth hash starts before the timeout.
- Process samples bind the exact test executable, argument, and worktree. The test thread remains runnable and consumes substantial CPU time.

The production content-reference function calls `blake3::hash` and formats its result.
The feature graph selects BLAKE3 1.8.5 with `pure` through Basalt's vendored `ucan-core`.
The declaration is `vendor/ucan/crates/ucan-core/Cargo.toml:18` at revision `89675cd4f585f837323c049e4a25f7b94c903038`.
BLAKE3's `Cargo.toml.orig:75-85` states that `pure` forces the pure-Rust fallback instead of the default x86_64 assembly implementations.

The samples establish setup cost for these runs, not every historical timeout or a complete CPU/wait breakdown.
A runtime reader found timer coverage limits around spawn and worker joins. The measured default failures do not reach those operations.
No producer-contract defect or production-process cause follows from that source review.

## Candidate and observed checks

`Cargo.toml` sets `[profile.dev.package.blake3]` to `opt-level = 3`.
Cargo's test profile inherits the development profile.
The compiler artifact record reports BLAKE3 optimization level 3, debug assertions enabled, and overflow checks enabled.
It retains features `default`, `pure`, and `std`. The compiler cohort and producer revisions remain unchanged.

All commands use the repository Nix shell, normal wrapper, two Cargo jobs, and the existing shared target directory.
The nextest commands use two test threads and eight-minute outer limits. No command-local profile override supplies acceptance.

| Check | Result |
| --- | --- |
| Default nextest, task 1491 | Exit 0. 1,646 passed, none skipped. Build 4m 13s, execution 83.064s |
| Existing CI nextest, task 1506 | Exit 0. 1,646 passed, none skipped. Build 0.68s, execution 88.512s |
| Workspace/all-target Clippy, task 1509 | Exit 0 with `-D warnings` |
| Core suite, task 1532 | Exit 0. 370 tests and seven doc tests pass |
| Compiler artifact read-back, task 1533 | Exit 0. Effective BLAKE3 profile and features retained |
| Locked all-feature metadata, task 1548 | Exit 0. Complete JSON read-back has 769 packages and four unchanged Radicle identities |
| Formatting, task 1554 | Exit 0 |
| Cairn validation/tasks, whitespace, and protected inputs, task 1555 | Exit 0. Structural checks only |
| CI JUnit preservation, task 1557 | Exit 0. Byte comparison passes |
| Full Nix, task 1558 | Exit 124 at the eight-minute outer deadline |

The unchanged native suite retains successful lifecycle/effect/recovery cases.
It also retains rejection cases for malformed, missing, mismatched, oversized, timed-out, and cancelled operations.
These results do not establish an all-feature test matrix, VM/fault acceptance, production readiness, or a numerical speedup for arbitrary workloads.

## Review and preservation

One bounded reader completes task 1501 with twelve reads/searches and no concrete configuration defect.
Its two internal passes are correlated. It runs no commands and grants no lifecycle approval.
The coordinator verifies the Cargo profile contract, exact diff, resolved feature graph, and compiler artifact record.

A separate heading-parser reader completes task 1391 with no concrete defect in its requested scope.
The coordinator verifies the unchanged parser and corrects two citation locations: CAS boundary is line 46, and verification is line 66.
The reader's original output remains intact. It supplies no Nix or lifecycle acceptance.

A Git quota error blocks one report update. The prior report remains intact.
Emergency evidence stays outside disposable worktrees on the home filesystem, then returns to the primary campaign after space becomes available.
The coordinator does not delete data to free disk space or change quotas or shared caches.
The native executable is absent again during the second diagnostic build. Its removal cause remains unproven.

A later bulk Pueue export returns empty objects for 22 older diagnostic tasks.
The native status query also lacks the sampled old tasks. The removal cause remains unknown.
Those empty objects are failed receipt exports, not terminal receipts.
Direct logs, exit files, source archives, and earlier exports remain available.
The current optimization commands have nonempty full Pueue exports.

The first restoration check passes content hashes but rejects the changed modification time.
The corrected restoration retains that failure, restores the archived timestamp, and passes the full original archive comparison.
The diagnostic source remains archived. The candidate contains no timing prints.

## Remaining acceptance

Full Nix, strict Octet, lifecycle acceptance, combined-source validation, and the wider feature/runtime matrix remain separate obligations.
The current Nix run reports cache timeouts, then starts a Nix dependency build for Cargo 1.95.0.
It reaches the outer deadline during compilation. It does not produce a current Tracey verdict or a full-check pass.
The earlier six dangling Tracey references remain unresolved. Their historical verdict does not describe this run's terminal cause.

Cairn reports `legacy_default`, no installation receipt, and empty acceptance/review receipt identifiers.
The Cargo change now records five completed implementation tasks. Its final acceptance task remains open.
Historical input manifests remain bound to their historical source. This report does not rewrite them.
No sync, archive, integration, push, deployment, or whole-Molten completion follows from these checks.
