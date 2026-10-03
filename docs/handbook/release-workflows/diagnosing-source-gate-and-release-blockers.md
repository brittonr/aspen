# Diagnosing source-gate and release blockers

Mode: Troubleshooting

Begin with the earliest failed boundary, not the last line of terminal output. Preserve the command inputs, candidate identity, available canonical artifacts, and diagnostics. This guide is source-checked, not an executed incident report; no runtime verification was performed for this batch. The known development-shell attempt failed on unsupported `git+rad` input. That is an environment/materialization blocker, not evidence that a release validator passed or denied.

## Symptom: the tool never reaches validation

**Discriminating evidence:** separate Nix input evaluation, compilation, Nickel execution/export, repository parsing, and the pure decision. The release-policy shell loads the profile and observes the repository before calling the core. An error that says Nickel could not execute or rejected the profile can occur without any `report_blake3`.

**Safe next action:** inspect the pinned tool configuration and supported transport requirements. Record the unsupported-input failure at that stage. Do not repeat the same failed shell attempt as confirmation, substitute dependency overrides as release evidence, or claim a smoke run that never reached the executable.

**Stop condition:** the reviewed toolchain or source cannot be materialized. Source review can continue, but release execution evidence remains unavailable.

## Symptom: an evidence-source argument fails

**Discriminating evidence:** the release-policy parser accepts `ID=PATH`, rejects empty halves and duplicate IDs, and only inserts default roots when no evidence-source argument was supplied. Providing one explicit source does not implicitly supply the second.

**Safe next action:** compare supplied IDs with `valence-integrity` and `octet-cutover` in the checked profile. Inspect the actual path roots and configured archive-relative paths. Absolute roots avoid ambiguity when `--root` differs from the working directory.

**Stop condition:** the expected reviewed archive cannot be located. A different checkout with similarly named task files is not a substitute.

## Symptom: dependency validation reports drift or missing evidence

| Evidence | What it discriminates | Safe next action |
| --- | --- | --- |
| `ManifestDrift`, `LockDrift`, `NixDrift` | Which representation differs from the expected row | Review that representation and the coordinated update procedure |
| `DuplicatePackageIdentity`, `NonCanonicalValence` | Graph identity conflicts or canonical authority mismatch | Inspect all matching resolved identities; do not hide the extra source |
| `MissingArchiveReceipt`, `InvalidArchiveReceipt` | Missing archive/bytes or unacceptable archive linkage | Inspect configured paths, byte hashes, size, and locked evidence revision |
| `MissingDistributionArtifact` | Required project-policy file absent | Restore the reviewed distribution input through its owning workflow |

The archive observer converts file-hashing errors into an absent hash observation. A missing hash therefore does not distinguish unreadable files, missing files, and files over its 1,048,576-byte bound. Inspect those conditions before deciding that the expected digest is stale. Do not edit expected hashes merely to match arbitrary local bytes.

Stop if the intended dependency revision or evidence revision is not reviewed. Local build success does not resolve identity drift.

## Symptom: a passing Octet receipt is rejected downstream

**Discriminating evidence:** source validation requires more than a receipt decision of `pass`. The inspected implementation checks supported consumer/scope, subject-reference shape, current workspace metadata, strict-profile checks, required artifact refs, canonical ref shape, clean finding counts, and source-coverage evidence. It compares current config/profile metadata and requires fingerprint/object-corpus checks.

A quarantine-oriented or otherwise limited result must not be presented as strict clean source-gate evidence. Inspect the canonical validation checks and diagnostics rather than searching terminal output for the word “pass.”

**Safe next action:** retain the old receipt and determine whether the blocker is stale metadata, wrong scope, missing artifacts, or actual findings. Correct the owning input or code, then regenerate evidence through the governed Octet workflow in a fresh output location. Do not loosen the profile or erase findings to make the gate accept.

**Stop condition:** strict current evidence cannot be produced for the required scope. A release-profile reference to the old receipt does not fix that blocker.

## Symptom: release-profile validation denies or produces no artifact

**Discriminating evidence:** inspect missing/placeholder candidate and evidence refs, missing required stack provenance, duplicate policy hashes, and `stale-generated-profile` diagnostics. Expected and actual export references must represent independently obtained values.

The CLI emits a constructed validation value before returning a decision-denial error. However, text/bound validation may return an error before construction, and artifact IO may fail. The operator runbook's broad deny-output description must therefore be read with this implementation boundary: absence of output is not itself a serialized deny result.

**Safe next action:** preserve a written denial when present; otherwise record the earlier failure separately. Resolve malformed input or output-path access without manufacturing a receipt. Use a fresh isolated output file on the next justified attempt.

**Stop condition:** underlying references cannot be inspected or tied to the intended candidate. Syntactic acceptability alone is insufficient.

## Worked failure: two green summaries, no releasable candidate

Suppose a dependency check passes and a release-profile conformance fixture passes, while the candidate's current source gate is unavailable. The first result addresses dependency observations. The second exercises validator wiring using deterministic fixture references. Neither fills the missing current source-gate obligation.

Record all three states separately. Keep the candidate blocked for the corresponding claim, even if the review packet contains other useful evidence. Do not copy fixture references into the candidate profile or relabel unavailable execution as diagnostic success. The [readiness companion](../../technical/proof/release-readiness-and-proof-scope.md) explains the scope boundary; this procedure preserves it during diagnosis.

## Sources

- [Handbook](../README.md)
- [Reproducible dependency workflow](../../reproducible-dependencies.md)
- [Production operator runbooks](../../production-operator-runbooks.md)
- [Readiness companion](../../technical/proof/release-readiness-and-proof-scope.md)
- [Release-policy parser and failure ordering](../../../crates/molten-release-policy/src/main.rs)
- [Archive observation and bounds](../../../crates/molten-release-policy/src/observe.rs)
- [Dependency diagnostic taxonomy](../../../crates/molten-core/src/release_dependency.rs)
- [Octet source-gate checks](../../../src/octet/parts/gate/p004/body.rs)
- [Gate artifact emission](../../../src/cli/evidence/gate/ops.rs)
