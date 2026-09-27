# Preparing a reviewable change

Mode: Review checklist

Use this checklist to assemble evidence a reviewer can evaluate without guessing your environment, selected tests, or intended claim. It complements the [proof workflow](../../proof-workflow.md), which governs proof-affecting changes, and the [dependency update policy](../../reproducible-dependencies.md). It does not introduce a new release gate. Return to the [Handbook](../README.md).

This page was prepared by source review, not by executing the checks it discusses. No runtime pass, regenerated dependency plan, or release artifact is claimed here. A review submission should make the same distinction between work actually observed and suggested follow-up commands.

## Scope and observable behavior

- [ ] Can the reviewer state the consumer-visible change in one sentence? Name the input, expected result, and boundary that rejects invalid input. “Refactored tests” is not enough when their authority or lifetime changes.
- [ ] Are all changed callers accounted for, including path aliases and included source parts? Supply the actual module route where physical filenames do not match Rust names.
- [ ] Is unchanged behavior explicit? For an export change, distinguish accepted cross-workspace destinations from forbidden source substitution rather than describing both as “cross-workspace access.”
- [ ] Are pure decisions, shell effects, fixtures, and live adapters labeled separately? A fixture pass must not be presented as live deployment support.
- [ ] Are new abstraction, dependency, or feature changes necessary for the claim? Avoid making unrelated environment repairs part of a small behavior review without explaining their coupling.

Useful evidence is a short map from changed behavior to owning implementation and relevant callers. The [root library](../../../src/lib.rs) and [test-support includes](../../../src/test/support.rs) demonstrate why reviewers should follow declarations rather than infer ownership from a directory name.

## Environment and reproducibility

- [ ] Does the submission name the checkout, Rust toolchain, package, target, features, and test runner used? Compare these with [the toolchain file](../../../rust-toolchain.toml), [manifest](../../../Cargo.toml), and [nextest configuration](../../../.config/nextest.toml).
- [ ] Are failures classified by the earliest reached boundary: shell evaluation, fetch, compile, test selection, assertion, or evidence validation?
- [ ] Are unexecuted checks and inaccessible dependencies recorded as blockers rather than pass results? An unsupported input transport says nothing about assertions that never ran.
- [ ] If local source overrides were used, are they visibly development-only? A convenient sibling checkout cannot stand in for reviewed release source identity.
- [ ] For a dependency update, do the reviewed profile, manifest revisions, lockfiles, Nix inputs, metadata-free source hashes, and both generated plans agree? Follow regeneration through the owning tools; do not hand-edit a lock to make a diff look complete.

The [dependency cohort companion](../../technical/engineering/dependency-cohorts-and-reproducible-builds.md) explains why these representations are separate obligations. The governing policy, not this checklist, supplies the update sequence.

## Test selection and strength

- [ ] Did the intended test actually run? Include selected test identities, not just a successful command exit or an appealing profile name.
- [ ] Is there a positive control for each important denial? A validator that rejects everything must not satisfy the entire evidence story.
- [ ] Does each assertion observe behavior rather than copied wiring, rendered wording, or a helper echoing its input?
- [ ] If a pure invariant changed, would generated inputs add useful coverage beyond examples? Apply the proof workflow's Hegel guidance where appropriate rather than adding properties ceremonially.
- [ ] Are retry-assisted exploratory results separated from deterministic evidence? The checked-in exploratory profile permits one retry and passes flaky results.
- [ ] Are platform and optional-feature exclusions visible? A VM-name test partition is not the same observation as booting the NixOS VM check.

## Filesystem authority and artifact lifetime

- [ ] Does a test obtain its root through the reviewed workspace shell instead of choosing a predictable temporary path?
- [ ] Does each operation receive the narrowest practical role root, with adversarial setup retained in the test shell?
- [ ] Does a child operation retain a workspace or root guard for its whole lifetime? `ChildProcessPlan` contains a path and label; the plan alone is not the RAII owner.
- [ ] Are selected retained artifacts exported into an explicitly supplied output root whose lifetime matches the retention claim?
- [ ] Are host paths confined to diagnostic/process setup use rather than canonical observations? Is cleanup limited to owned state, with no prefix-based ambient deletion?

The governing migration is scoped. Do not claim every historical helper has already migrated, that typed roles sandbox arbitrary native code, or that RAII survives process termination.

## Worked review case: export ownership

Suppose a patch changes the source ownership check in `export_selected`. A reviewable submission points to the [implementation](../../../src/test/parts/support/p000/body.rs), the positive asynchronous export test, and `wrong_workspace_and_invalid_export_are_denied` in [the tests](../../../src/test/parts/support/p002/body.rs). It explains that the positive fixture copies literal `receipt` bytes to a separate workspace's output, while the negative case rejects a foreign source with `PermissionDenied`.

The review should reject “canonical receipt verified” as a description of that fixture: the payload is not a canonical Preserves receipt. The test checks a BLAKE3 prefix, not an independently recomputed digest. This narrower, accurate claim is more useful than overstating a passing assertion.

## Proof and handoff

- [ ] For proof-affecting work, are proof claims, non-claims, assumptions, positive/negative evidence, regeneration steps, and required canonical references present?
- [ ] Are rendered logs and JUnit treated as review aids rather than substitutes for required canonical receipts?
- [ ] Does a documentation-only change use an explicit supported exemption rather than fabricated execution evidence?
- [ ] Do documentation and command examples match the inspected source, with blocked or unexecuted steps marked honestly?

Canonical Preserves plus BLAKE3 define relevant product identities, not Rust layout. Evidence receipts describe observations; they do not themselves grant mutation or release authority. End the handoff with the exact exercised coverage and remaining blockers, not a broad readiness claim.

## Sources

- [Handbook](../README.md)
- [Proof workflow](../../proof-workflow.md)
- [Reproducible dependencies](../../reproducible-dependencies.md)
- [Test workspace authority](../../test-workspace-authority.md)
- [Workspace lifetime companion](../../technical/engineering/test-workspace-lifetime-and-authority.md)
- [Export implementation](../../../src/test/parts/support/p000/body.rs)
- [Workspace behavior tests](../../../src/test/parts/support/p002/body.rs)
- [Nextest profiles](../../../.config/nextest.toml)
