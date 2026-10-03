# Release Readiness and Proof Scope

Release review combines evidence with different subjects, execution environments, and limits. Its central discipline is preserving those differences rather than interpreting every passing artifact as production readiness. This article assumes the [proof workflow](../../proof-workflow.md), [distributed-testing evidence contract](../../distributed-testing.md), and [replay coverage readiness](../../replay-coverage-readiness.md). It explains the inspected profile and matrix validators; it does not announce a release decision. Related articles are listed in the [Technical companion](../README.md).

## Evidence scope does not grow by aggregation

Deterministic simulation concerns a model with explicit topology, scheduling, seed, fault plan, commands, and virtual time. Local multiprocess evidence adds observations from actual local child processes. VM evidence addresses platform integration, and live or soak evidence introduces still different environmental conditions. The distributed-testing contract explicitly keeps these scopes separate: a simulation result is not WAN evidence, and a local multiprocess receipt does not replace a NixOS VM check.

The same distinction applies inside VM reporting. `fixture-metadata`, `executable-vm`, `aggregate-index`, and `diagnostic-only` are different evidence scopes described by the governing document. An index containing a VM-shaped fixture does not thereby contain an executed VM result. Unsupported support is recorded as unavailable, skipped, or denied evidence; it cannot satisfy a claim that requires execution on that platform.

CI/release pass evidence also uses zero retries under that contract. An exploratory rerun can help identify a failure, but “eventually passed” is a different observation from deterministic pass evidence. Retry, duplicate suppression, and replay each address particular boundaries; none alone establishes exactly-once external effects or production reliability.

## What release profile validation checks

`ReleaseProfileInput` identifies a profile and tier, optional candidate reference, evidence references, generated-export freshness values, whether stack provenance is required, accepted Valence policy hashes, and caveats. The [profile validator](../../../src/prod/release/parts/profile/p000/body.rs) recognizes development, pilot, and release tiers. Release-specific checks require a non-placeholder candidate and the supplied evidence-reference fields, including source gate, policy, Octet, Cairn, stack provenance, and production profile.

For release tier, stack provenance cannot be optional. Expected and actual generated-export references must both be supplied, be valid references, agree with each other, and avoid recognized placeholders. Accepted Valence policy hashes cannot be absent for release; duplicate and recognized placeholder values produce diagnostics. These checks describe the supplied profile values. They do not execute a source gate, contact a provenance service, or cryptographically validate every referenced artifact.

The validator sorts and deduplicates diagnostics, derives pass or deny, serializes `release-profile-validation-v1`, and computes its canonical reference. Its built-in caveat says the result is release-review evidence only and does not grant release eligibility by itself. A profile passing these checks is therefore a structurally acceptable review input, not proof that the referenced candidate is deployable under all operational conditions.

## Replay coverage is readback, not execution

The [replay matrix implementation](../../../src/deterministic/parts/replay/p005/body.rs) distinguishes `Deterministic`, `Recorded`, `DiagnosticOnly`, and `NonReplayable` rows. Each row names subsystem and workflow and can reference a fresh run, replay verification, second fresh run, negative evidence, replay index, and caveats.

Deterministic and recorded rows require the first four evidence references. Diagnostic-only rows cannot supply replay verification as deterministic evidence and need caveats. Non-replayable rows cannot report fresh-run or verification references as replay pass evidence and also need caveats. Duplicate row identities are diagnosed. The matrix's canonical record includes an evidence-only check.

Those checks establish supplied-field consistency and presence. They do not perform replay or compare the referenced run contents inside the matrix validator. The governing readiness page discusses stale references; the inspected matrix validates reference syntax and row structure without dereferencing objects to establish semantic freshness. Readback acceptance should not be described as independent verification of every referenced run. Actual replay receipts and their consuming gates retain that responsibility.

## Worked readiness review

Consider an illustrative candidate with a valid release profile, complete deterministic replay rows, and a requested VM fault check that the host cannot execute. The profile result says its reference and freshness constraints were satisfied. The replay matrix says its declared rows have admissible evidence shapes. Neither result fills the missing executable VM scope.

A faithful review records the unavailable VM evidence and retains the limitation on any platform-dependent claim. Adding a caveat to a non-replayable row can accurately describe that row, but it cannot change it into deterministic coverage. Replacing the VM result with a fixture-metadata aggregate changes what is indexed, not what was executed. Similarly, a stack adapter report can establish vocabulary compatibility without establishing release or transport authority.

## Verification guidance and limits

Review each claim against the cheapest evidence scope that actually exercises it, then inspect any additional required platform scope. Suggested focused checks include the governing workflow's `cargo test release_profile --lib`; the distributed-testing document provides simulation and VM command surfaces. These commands are suggestions only, not executions reported here.

Follow canonical references from candidate to profile, child receipts, replay observations, and negative evidence. Read summaries last, preserving unavailable states and declared variance rather than normalizing them into success. This article supplies no deployment approval, production-readiness verdict, formal correctness proof, or substitute for subsystem policy, authority, retention, resource, and lifecycle gates.

## Sources

- [Proof workflow](../../proof-workflow.md)
- [Distributed testing evidence](../../distributed-testing.md)
- [Replay coverage readiness](../../replay-coverage-readiness.md)
- [Valence stack evidence adapter](../../valence-stack-evidence-adapter.md)
- [Release profile validation](../../../src/prod/release/parts/profile/p000/body.rs)
- [Replay coverage matrix validation](../../../src/deterministic/parts/replay/p005/body.rs)
- [Technical companion](../README.md)
