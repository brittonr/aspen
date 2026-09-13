# Native Tracey input correction

## Goal and limits

Make the existing Tracey guard and classifier read the current native accepted-specification tree.
A missing, non-directory, empty, or unmarked requirement tree must reject explicitly.
Valid fixtures must expose their requirement identifiers and specification paths.
Unknown source references and baseline drift must still reject.

This correction does not establish implementation coverage, change accepted requirements, or admit new debt.
Do not change `inherited-debt-baseline.txt`, its historical metadata, classification claims, producer pins, or lockfiles.
Do not include active change definitions as accepted coverage merely to remove a dangling-reference error.
No sync, archive, publication, or mainline integration belongs to this correction.

Molten Tracey maintainers own the corrected input boundary and its tests.
The current consumer is the `inherited-tracey-debt` Nix check.
Its immediate target is correct input selection and explicit rejection, not a manufactured full-gate pass.
Existing pure marker parsing, classification, and baseline comparison retain their behavior.
Filesystem access stays in the existing script shells.

## Source and evidence

The coordinator fetched `origin/molten` at `87c289bf68c45987d2d79572de1ca458a7ca662f`.
It created `implementation-tracey` on `completion/tracey-native-inputs-20260913` from that ref.
A fast-forward preserved the inspected local source through merge `7c52757bbd9bfb44574a779eba25d58161c9e754`.
The clean starting tree is `b0cf08d651ef1e81be199b54aeb576143ddfe30c`.
The separate retry worktree remains frozen during its checks.

The current Nix failure and bounded advisory review are recorded in `combined-validation.md`.
Both tools select `cairn/specs`, while native source definitions reside in `.cairn/specs`.
The exact failed derivation contains the current specification. Its source filter is not the demonstrated cause.
The old guard returns an empty requirement set for a missing path.

The historical baseline names 2,508 requirements and 1,924 uncovered entries at its recorded origin revision.
Current discovery names 2,368 accepted requirements. These snapshots are not interchangeable.
A correct path alone cannot establish that the historical debt set still matches the current source.
F12 definitions also remain active, not accepted. The correction must expose these later checks rather than erase them.

## Bounded approach registry

| Family | Mechanism | Evidence | State |
|---|---|---|---|
| Source filter | Verify the exact Nix source contains the native specification | The selected store source contains `.cairn/specs/world-commit/spec.md:526` | Rejected as the demonstrated cause |
| Root selection | Use the native accepted root and reject absent input | Existing constants and missing-path behavior | Source-backed cause, correction pending |
| Coverage drift | Compare current findings with unchanged historical debt | Current and historical requirement counts differ | Unresolved, no automatic baseline update |

Budget: one focused correction, twelve further source reads/searches, and one review round.
Each command keeps the existing eight-minute deadline and shared build limit.
Run existing tests first, then meaningful positive and negative input tests before the repair.
Retain each direct exit and transcript outside disposable worktrees.
The allowed outcomes are a validated input correction, an exact blocker, or budget exhaustion.

## Baseline

Task 600 ran the existing guard and classifier tests through the unchanged repository Nix toolchain.
Each suite passed four tests. The direct exit is 0.
Evidence: `logs/tracey-input-baseline.{log,exit}`.
The complete Nix baseline already failed with zero requirements and dangling references.

## Executed correction

The coordinator added four input tests to each script before changing either reader.
Each red run passed all four existing tests and failed all four new tests, with exit 101.
The failures showed an empty result for current native definitions and absent input.
`logs/tracey-input-red-inputs.b3` binds that test source.

Both readers now select `.cairn/specs` and require a real, nonempty specification directory.
Missing, legacy-only, file-valued, empty, and unmarked roots reject explicitly.
The shared test helper creates new task-owned temporary directories and never replaces an existing path.
It removes only the directories it created.
No parser, baseline comparison, classification rule, historical manifest, or accepted specification changed.

The corrected guard and classifier each pass eight tests through the repository Nix toolchain.
Positive cases preserve native definition identity and path and resolve a known source reference.
Negative cases retain unknown-reference rejection and duplicate rejection for baseline identifiers.
`logs/tracey-input-green.{log,exit}` records exit 0.
`logs/tracey-input-candidate-inputs.b3` binds the corrected source and unchanged policy inputs.

## Scoped Nix result

Task 650 ran the unchanged `checks.x86_64-linux.inherited-tracey-debt` check and returned exit 1.
The configured builder was `ssh-ng://root@aspen1.local`.
Cache connection failures preceded the build. They were not its terminal failure.
The derivation is `/nix/store/clv3hcls3w764f53kvp0yj983mfg1h8h-molten-inherited-tracey-debt.drv`.
The selected source is `/nix/store/l4djy69xvqhyi9ksvgrax0xrnaaw8dgq-source`.

All eight guard tests pass inside that Nix build.
The corrected guard then reports `requirements=2777`, `referenced=787`, `uncovered=1990`, `baseline_entries=1924`, and `dangling=10`.
These are this guard's lexical observations. They are not interchangeable with the frozen lifecycle inventory's 2,368 accepted-requirement entries.
The difference between those counting methods remains unresolved.

The ten rejected references are:

- `aspen.cas.boundary`
- `aspen.cas.contract`
- `aspen.cas.decision`
- `aspen.cas.verification`
- `molten.audit_f12.bounds`
- `molten.audit_f12.compatibility`
- `molten.audit_f12.saturation`
- `molten.audit_f12.validation`
- `molten.consensus.chaoscontrol_chain_observation`
- `molten.consensus.chaoscontrol_operation_identity`

The exact terminal diagnostic is `error: dangling traceability references are not permitted`.
The guard rejects before the remaining classification and baseline checks complete.
No complete Nix pass, baseline reconciliation, or path-bearing inventory migration is established.
The old baseline and classification metadata remain unchanged historical inputs.

## Independent review and scope decision

Task 652 completed with exit 0 after ten reads.
The read-only reviewer identified duplicate identifiers outside the debt baseline as a possible medium policy finding.
Its proposed case has two definitions, a direct reference, and an empty debt baseline.
The existing guard collapses duplicate identifiers, while classification checks only baseline identifiers.
The reviewer did not establish that this correction introduced that behavior.

The coordinator checked `.cairn/specs/project/spec.md:1008-1014` before changing policy.
The controlling requirement says:

> Molten MUST reject classification when a baseline identifier has no accepted definition or has more than one accepted definition.

The proposed outside-baseline case is beyond that explicit classification requirement.
The new negative test covers a duplicated baseline identifier, as required.
No global duplicate-rejection claim follows from this correction or these tests.
The coordinator therefore retained the existing classification rule rather than silently broadening policy.
This disposition does not prove global identifier uniqueness or reject the reviewer's source observation.

This reader used `--no-extensions` within the same five-minute deadline and completed normally.
It did not verify source hashes, execute tests, or grant lifecycle acceptance.
The source manifest and executed results are separate coordinator evidence.

## Remaining blocker

The corrected input boundary exposes unresolved references and debt that the obsolete root hid.
The declared budget permits this input correction, not automatic baseline growth, marker deletion, or early specification acceptance.
The next policy action needs reviewed requirement mappings and disposition evidence for the actual source tree.
No full-gate or Molten completion claim is made.

## Reproduction and retained payloads

Use the repository Nix environment and Rust 2024 edition to compile each standalone test crate.
The commands are `rustc --test tools/tracey/inherited_debt_guard.rs` and `rustc --test tools/tracey/inherited_debt_classifier.rs`, with separate output binaries.
Run both binaries. The exact process transcripts and direct exits reside in the log archive.
The scoped check command is:

```console
nix build .#checks.x86_64-linux.inherited-tracey-debt --no-link -L --no-write-lock-file --max-jobs 1 --cores 2
```

`evidence/tracey/native-input-correction/` retains this note, selected input identities, lossless logs, review text, and red source.
The red source was reconstructed from immutable parent files and the unchanged test modules.
Every reconstructed byte passed the original five-file red BLAKE3 manifest before archiving.
`inputs.b3` binds current selected code and protected policy inputs. `payloads.b3` binds the archives.
Historical baseline and classification files remain outside this new evidence directory and remain unchanged.
