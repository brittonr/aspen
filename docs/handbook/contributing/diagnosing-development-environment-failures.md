# Diagnosing development environment failures

Mode: Troubleshooting

Classify the earliest failing boundary before changing the checkout. A fetch failure, compiler failure, zero-test selection, assertion failure, and denied evidence gate require different remedies. Keep original diagnostics and the exact attempted command. This guide is source-checked; it does not report successful runtime verification. Return to the [Handbook](../README.md).

## Shell initialization rejects an input transport

**Symptom:** Nix fails before providing a development shell, for example with an unsupported `git+rad` input. An attempt in this handbook preparation session reached that error; it was not a workspace test failure, and it is not claimed to be fixed here.

**Discriminating evidence:** identify the input coordinate named in the diagnostic and compare it with the checked-in flake input/lock and the dependency profile. Record which Nix operation failed. Do not interpret the absence of test output as a failed assertion or assume that a Cargo feature switch removes every source-resolution prerequisite.

**Safe next action:** establish whether the available tooling supports the reviewed input transport and whether the intended source is reachable through the approved environment. Follow the [dependency policy](../../reproducible-dependencies.md) if an actual pin or transport change is needed. Preserve that review as a dependency change, not an invisible workstation workaround.

**Stop condition:** no supported source acquisition path has been established. Do not repeatedly execute the same failing setup, silently replace the source, or report checks downstream of shell initialization as run.

## Cargo cannot fetch a pinned Git dependency

**Symptom:** compilation never begins because a Git source cannot be obtained.

**Discriminating evidence:** the root and core manifests contain exact Git revisions, including SSH and other reviewed transports. [.cargo/config.toml](../../../.cargo/config.toml) enables CLI Git fetching. Distinguish authentication/access failure, transport support, and missing revision from a Rust error. Do not publish secrets while retaining diagnostics.

**Safe next action:** verify access to the exact approved source through the environment owner. If an explicit local sibling substitution is used for development, label the result accordingly. The [governing policy](../../reproducible-dependencies.md) says local Nix overrides do not replace reviewed release identities.

**Stop condition:** access or revision availability remains unresolved. Editing a manifest to a convenient branch, changing trust settings, or substituting another package identity is not a valid repair for release evidence.

## Compiler or runner differs from the repository contract

**Symptom:** feature-gate/compiler metadata errors, or nextest rejects configuration before running tests.

**Discriminating evidence:** compare the actual tool identity in the failure report with [rust-toolchain.toml](../../../rust-toolchain.toml) and the required nextest version in [.config/nextest.toml](../../../.config/nextest.toml). The library uses nightly tool attributes. The flake also contains explicit toolchain alignment for the unit2nix compiler and Clippy path; an ambient compiler is not necessarily interchangeable.

**Safe next action:** restore the declared tool selection through the supported environment, then resume at the previously blocked boundary. This is different from changing application code to appease an unintended compiler. Preserve build provenance when switching environments.

**Stop condition:** the correct tools cannot be supplied. Do not mark a different-toolchain run as equivalent or delete unrelated state hoping to change the result.

## The command succeeds but the intended test did not run

**Symptom:** a successful exit or profile report is offered as proof for behavior that is absent from the selected tests.

**Discriminating evidence:** inspect package and target selection, then the test-name filter. `fast-core` selects matching names inside `package(molten)`, not every member of the workspace. The export test in the [workspace test source](../../../src/test/parts/support/p002/body.rs) is not selected merely because its helper is foundational.

**Safe next action:** use the exact package, library target, and existing test name from [the focused-test guide](choosing-a-focused-test.md). Observe selection as well as completion. A feature-gated integration test additionally requires its declared feature; do not assume it participated in a default run.

**Stop condition:** selection remains empty or a prerequisite still blocks execution. Record no behavioral evidence from that attempt.

## A temporary workspace fails during construction

**Symptom:** the test fails before its meaningful input is written, or the child-process bridge reports an I/O error.

**Discriminating evidence:** [construction](../../../src/test/parts/support/p000/body.rs) obtains a capability temporary directory and immediately derives a diagnostic host path. The [Unix implementation](../../../src/test/parts/support/p001/body.rs) reads `/proc/self/fd/<descriptor>`; non-Unix explicitly returns `Unsupported`. On Unix without a usable descriptor path, the actual filesystem error propagates. This is a source-review qualification to the governing document's broader unsupported-host wording, not a reproduced portability bug.

**Safe next action:** distinguish workspace construction from source ownership rejection. Keep the host/environment failure separate from the test's intended behavioral assertion. For a substituted source root, `PermissionDenied` is instead the expected boundary exercised by `wrong_workspace_and_invalid_export_are_denied`.

**Stop condition:** the reviewed host bridge is unavailable. Do not add an ambient-path fallback, weaken ownership checks, or scan and delete temporary directories by prefix. Normal RAII cleanup does not promise cleanup after abrupt termination.

## A local build passes but dependency validation denies

**Worked failure case:** a Cargo revision is updated while the matching Nix identity remains old. A cached local build can succeed without establishing cohort agreement. A metadata-bearing prefetched source hash can also differ from the metadata-free source used by Nix builders.

Read the denial as a consistency failure, not a request to bypass the validator. The policy requires coordinated profile, manifest, lock, hash, and generated-plan updates. Resume release claims only after the prescribed validation succeeds on the reviewed identities; a build result alone cannot close that obligation.

## Sources

- [Handbook](../README.md)
- [Reproducible dependency policy](../../reproducible-dependencies.md)
- [Dependency cohort companion](../../technical/engineering/dependency-cohorts-and-reproducible-builds.md)
- [Workspace authority](../../test-workspace-authority.md)
- [Workspace lifetime companion](../../technical/engineering/test-workspace-lifetime-and-authority.md)
- [Host bridge implementation](../../../src/test/parts/support/p001/body.rs)
- [Toolchain file](../../../rust-toolchain.toml)
- [Nextest configuration](../../../.config/nextest.toml)
