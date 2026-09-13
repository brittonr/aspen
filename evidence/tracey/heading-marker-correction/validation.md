# Accepted requirement-heading recognition

## Result and owner

Both inherited Tracey tools now recognize the heading syntax already present in the accepted CAS specification.
The correction preserves all four exact CAS identifiers and resolves their existing source references.
Molten Tracey maintainers own the shared private parser and both script consumers.
The parser is pure. The existing scripts retain filesystem access, classification orchestration, and output effects.
No new dependency, port, public product API, accepted requirement, or policy rule is necessary.

The candidate starts from `077d7eacfaebf9cc0a59e32879d1792376375809`.
The retry-domain correction remains on its separate branch.
This correction does not complete F12, the Tracey debt gate, or Molten.

## Demonstrated defect

The accepted file `.cairn/specs/object-store-cas-coordinator/spec.md` defines:

- `aspen.cas.contract` at line 9.
- `aspen.cas.decision` at line 27.
- `aspen.cas.boundary` at line 46.
- `aspen.cas.verification` at line 66.

Each marker appears at the end of a requirement heading, inside a bracket wrapper.
The old readers recognized only lines that started with `r[` after whitespace removal.
The existing source references therefore appeared unresolved despite their exact accepted definitions.
No renamed replacement or new accepted text was required.

The original guard and classifier each passed eight tests before new tests were added.
Each red run then passed eleven tests and failed two, with exit 101.
The failures showed missing heading identity and an empty accepted-definition tree for valid heading input.
The patch and new test module in the log archive preserve that red source.

## Correction and negative boundaries

`tools/tracey/definition_marker.rs` owns the shared pure recognition function.
The guard and classifier both consume it.
It preserves existing standalone-marker behavior and recognizes a trailing `[r[identifier]]` wrapper on a nonempty requirement heading.
It rejects reference verbs, arbitrary prose, other headings, malformed wrappers, empty titles, and ambiguous multiple markers.
Heading modifiers cannot conceal extra brackets.
The existing standalone version-suffix behavior remains unchanged.

Positive input tests preserve exact identifiers, source paths, and line numbers.
Negative input tests retain unknown-reference rejection and duplicate rejection for baseline identifiers across both definition forms.
Existing malformed-baseline, absent-root, empty-root, and conservative-classification tests also remain intact.
This bounded lexical parser is not a general Markdown validator or proof of globally unique requirement identities.

## Executed checks

| Check | Result |
|---|---|
| Existing guard and classifier baselines | Exit 0, eight tests each |
| New tests against unchanged readers | Exit 101 each, eleven passes and two intended failures |
| Corrected standalone suites with `rustc -D warnings` | Exit 0, thirteen tests each |
| `clippy-driver -D warnings --test` for both scripts | Exit 0 |
| Rust formatting and staged whitespace | Pass |
| Cairn strict structural validation | Exit 0, all 55 active changes |
| Scoped inherited-debt Nix check | Exit 1 after thirteen guard tests pass |

The Nix derivation is `/nix/store/zr7p7pwwyj56bkr1gzsf66n0wdk7j29p-molten-inherited-tracey-debt.drv`.
Its source is `/nix/store/1imwr6dbazvp0qyvkrhi1wcz07nz94qw-source`.
The configured builder is `ssh-ng://root@aspen1.local`.
The guard reports `requirements=2781`, `referenced=791`, `uncovered=1990`, `baseline_entries=1924`, and `dangling=6`.
The six unresolved identifiers belong to the active F12 and ChaosControl changes.
The exact terminal diagnostic remains `error: dangling traceability references are not permitted`.
Historical debt and classification inputs remain unchanged. The build rejects before later baseline and classifier checks complete.
No full Nix pass follows from this narrower correction.
Cairn selected `legacy_default` policy without an installation receipt.
Its structural receipt is `bbd1ee1194a1621e9017229af3edeb1824a95a14e4da32dcb795fc70e7158d7b`.
This structural result does not bind product behavior or grant acceptance.

## Definition inventory reconciliation

The accepted specification tree has no tracked difference from frozen revision `bd1f7f78d1d13e9d1465ec7bfd931124c39401be`.
The campaign's original extractor selected one identifier per `### Requirement:` section.
Its 2,368 count therefore describes sections, not every named traceability obligation.
The accepted tree also contains 413 additional identifiers under those sections.
Some identify extra normative clauses. Others identify scenarios.
The old standalone-only guard found 2,777 identifiers because it omitted the four CAS heading identifiers.
The corrected guard and a separate anchored source index both find 2,781 unique identifiers.
These counts do not establish implementation coverage or acceptance.

The old `requirements.json` also has an invalid trailing suffix.
The coordinator preserved its complete bytes rather than silently trimming or accepting them.
Its corruption cause remains unproved.
A fresh `accepted-marker-index.json` passes complete JSON read-back and labels every definition `UNKNOWN`.
That index contains source paths and line numbers. It does not replace the unresolved lifecycle metadata or acceptance mapping.
The persistent campaign retains the original malformed artifact, normalization code, exact source selection, and controls.

## Review limits

The definition-provenance reader completed with exit 0 within fifteen reads.
It identified four exact accepted CAS definitions and six active, unsynced definitions.
The coordinator checked the CAS source and reproduced the parser failure before repair.

A new independent parser review did not start.
The first invocation rejected an unknown account-qualified provider with exit 1.
The corrected provider invocation also exited 1: `Codex error: The usage limit has been reached`.
Both logs remain intact. Neither is a completed parser review.
The coordinator's local audit is not an independent pass.
It covers the explicit positive and negative cases above, not arbitrary Markdown or uninspected lifecycle behavior.

## Reproduction and evidence

All product checks use the repository Nix environment without lockfile writes.
Each command retains the eight-minute deadline and serialized build limit.
Standalone test commands use the Rust 2024 edition and `RUST_TEST_THREADS=2`.

```console
rustc --edition=2024 -D warnings --test tools/tracey/inherited_debt_guard.rs -o GUARD_TEST
rustc --edition=2024 -D warnings --test tools/tracey/inherited_debt_classifier.rs -o CLASSIFIER_TEST
nix build .#checks.x86_64-linux.inherited-tracey-debt --no-link -L --no-write-lock-file --max-jobs 1 --cores 2
```

`inputs.b3` binds the selected source and unchanged policy inputs.
`logs.tar.gz` retains lossless process transcripts, direct exits, the red patch, and the red test module.
`inventory.tar.gz` retains the corrected index, source selection, normalizer, malformed historical index, and diagnostic observations.
`accepted-specs.b3` binds every tracked accepted-specification source file.
`payloads.b3` binds the retained payloads.
The persistent campaign keeps the original files outside disposable worktrees.
No accepted-spec sync, archive, debt growth, marker removal, mainline integration, or push occurred.
