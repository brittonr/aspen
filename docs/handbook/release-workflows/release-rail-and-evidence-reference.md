# Release rail and evidence reference

Mode: Reference

Use this reference to identify the owner and interpretation of an input before adding it to a review packet. It is a source-checked inventory, not an execution record. No listed rail was run for this documentation batch. The [readiness companion](../../technical/proof/release-readiness-and-proof-scope.md) explains why combining evidence does not enlarge its scope.

## Rail lookup

These are configuration attribute names or CLI surfaces, not complete executable recipes. Execution requires the reviewed source, available toolchain, actual evidence, and an isolated output destination.

| Rail or surface | Owner | Input and result | Does not establish |
| --- | --- | --- | --- |
| `release-dependency-profile` flake check | `flake.nix`, release-policy executable | Typed dependency profile; repository observations; upstream archive inputs; textual decision/report hash | Binary reproducibility or release eligibility |
| `git-source-hash-binding` flake check | `flake.nix`, source-hash workflow | Recorded Git-source hashes and generated plans | Upstream correctness or runtime behavior |
| `release-profile-validation` flake check | `flake.nix` | Positive and negative supplied-reference fixtures | Current evidence for a real candidate |
| `release-candidate-binding` flake check | `flake.nix`, readiness builder | Declared artifact/source pairs, including wrong-candidate negatives | Truth of external artifacts |
| `production-profile-fixtures` flake check | `flake.nix`, production Nickel contracts | Candidate-customized export and negative contract fixtures | Adapter readiness or deployment authority |
| `molten test gate release-profile` | Gate CLI and release-profile validator | Supplied profile fields to `release-profile-validation-v1` | Execution or authentication of every referenced artifact |
| `molten test octet source-gate validate` | Octet CLI and source validator | Gate receipt, consumer, subject, optional scope to validation value | A substitute for downstream operation admission |

CLI route ownership starts at [main aliases](../../../src/main.rs), proceeds through the [included test command enum](../../../src/main/root/parts/command/p000/body.rs), then the [gate declaration](../../../src/cli/evidence/gate/command.rs) or [Octet declaration](../../../src/cli/ops/octet/command.rs). Similar names do not mean these commands consume interchangeable records.

## Dependency input fields

| Profile field | Meaning | Observing owner |
| --- | --- | --- |
| `manifest_dependency` | Key in the selected manifest dependency section | `observe_dependency` |
| `package_name`, `package_version` | Resolved package identity sought in the lockfile | `find_lock_identity`, graph identity collection |
| `source_coordinate`, `immutable_revision` | Reviewed upstream coordinate and immutable pin | Manifest/lock observations compared by core |
| `nix_input` | Named locked Nix input | `flake_input_revision` |
| `transport_policy` | `https`, `private-radicle`, or `ssh-pinned-nix-archive` | Profile conversion and pure policy validation |
| `disposition` | `runtime`, `optional-runtime`, or `development` | Profile conversion; manifest section selection |
| Archive `id`, `archive_path`, `evidence_files` | Evidence-root selection, directory, expected file hashes | Archive observer |
| Distribution `notice_artifacts`, `source_export_artifacts` | Project-required paths | File-presence observation |

The observer reads the selected root manifest and its lockfiles. Do not generalize that implementation into an independently fetched audit of every source repository. Archive evidence hashes cover configured bytes; distribution presence is a weaker observation and must be described as such.

## Release-profile field groups

| Fields | Release-tier review expectation | Implementation boundary |
| --- | --- | --- |
| `profile_id`, `tier`, `candidate_ref` | Named release review and real candidate reference | Text, tier, and reference checks |
| `source_gate_ref`, `policy_ref`, `octet_ref`, `cairn_ref` | Present, non-placeholder references with inspected underlying evidence | Validator does not execute the producers |
| `stack_provenance_ref`, `stack_provenance_required` | Present provenance and required status | Does not itself confer provenance trust |
| `production_profile_ref` | Reviewed profile for the intended candidate | Does not start adapters |
| `expected_generated_export_ref`, `actual_generated_export_ref` | Independent values that agree | Equality is checked; generation is not performed |
| `accepted_valence_policy_hashes` | Reviewed policy hashes, without duplicates or recognized placeholders | Placeholder checking is not proof of policy acceptance |
| `caveats` | Explicit limitations retained with the result | Caveats do not repair absent authority |

The profile implementation bounds supplied hashes at 64 and caveats at 128. Its validation reference comes from canonical Preserves hashing, not Rust struct layout. Parsing and field-bound errors can precede value construction, so not every unsuccessful invocation yields a deny artifact.

## Artifact interpretation

| Artifact or identity | Keep as | Never relabel as |
| --- | --- | --- |
| Dependency `report_blake3` | BLAKE3 of sorted textual report material | A Preserves receipt, binary digest, or approval |
| Release-profile validation value | Evidence about supplied review fields | Candidate execution proof |
| `prod-release-candidate-gate-v2` | Declared candidate-evidence composition | Independent authentication of external results |
| Octet gate/source-validation value | Evidence at its stated gate and scope | Blanket mutation permission |
| Profiler `.fxt` | Machine-local development observation | Release-readiness, Cairn, Valence, or determinism evidence |
| Rendered logs and summaries | Diagnostic navigation aids | Canonical receipt identity |

## Worked lookup: one row, three obligations

A reviewer sees a changed `valence` manifest declaration and a successful local build. First locate the profile row: its package is `valence-core`, not `valence`. Next inspect the Cargo and Nix locked revisions against the same reviewed revision. Finally inspect the configured Valence and Octet archive bytes and the source-hash/build-plan rail separately.

A matching Git pin does not guarantee a matching metadata-free NAR hash. A dependency report does not supply current Octet source-gate evidence. A passing release-profile fixture does not bind the actual candidate. The correct review packet retains these distinctions instead of compressing all three into “release checks passed.”

## Sources

- [Handbook](../README.md)
- [Reproducible dependency workflow](../../reproducible-dependencies.md)
- [Proof workflow](../../proof-workflow.md)
- [Readiness companion](../../technical/proof/release-readiness-and-proof-scope.md)
- [Dependency profile](../../../config/release-dependencies/profile.ncl)
- [Repository observation implementation](../../../crates/molten-release-policy/src/observe.rs)
- [Release-profile implementation](../../../src/prod/release/parts/profile/p000/body.rs)
- [Release rail definitions](../../../flake.nix)
- [Profiler artifact roles](../../../src/profiling.rs)
