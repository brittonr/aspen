# Final gate follow-up

This report supersedes the gate status in `../policy-resume/report.md`.
The policy repair is committed as `a7548ce58a6cd1a45ef694cbd101106cb3a5de49`.
The retry implementation remains at `fbd9ca63f5ce68a5b2007793b09913347468d68f`.

## Documented strict source gate

The README workflow produced fresh configured and library artifacts.
The configured check reported 6,720 findings and `warning-only` status.
The library check reported 2,795 findings and `warning-only` status.
These counts overlap. Do not add them or treat them as counts of unique defects.

The documented `cargo run` commands need `--bin molten` because this workspace has multiple binaries.
The first attempt stopped at that CLI error. The corrected invocation completed artifact import and reached the strict gate.
Artifact import passed. The `strict-ci` gate denied the imported artifacts and exited 1.
No quarantine profile, warning baseline, or disabled lint was added.

Canonical receipt references:

- Artifact ledger: `blake3:29c6afac24c88781be0a17ee6376c8241da3b11e3e2b39b79ca3d2bf30f7b33f`.
- Denied strict gate: `blake3:9ec72086c7d01f537a42dd16f2ede22fc77c410ec2b8ec564a25dabc1938dd25`.
- Remediation inventory: `blake3:bdbad91739161b16c7273055cab417f47437414d35c8c239ca1668288adb28b1`.

A passing artifact import establishes neither clean source nor release eligibility.
The separate all-feature Cargo/Rad panic remains reproducible, as recorded in the policy report.
Fixing that panic alone would not clear this strict gate.

## Nix result and Tracey reader repair

The full Nix run used a thirty-minute budget against the committed policy repair.
It reached terminal failure with exit 1, rather than exhausting that budget.
The reported failed check was `inherited-tracey-debt`.

Both Tracey readers still selected `cairn/specs`, although this repository uses `.cairn/specs`.
The old guard therefore read no accepted requirements and reported the current references as dangling.

The guard and classifier now select `.cairn/specs`.
They reject a missing root or a root that is not a directory.
They do not fall back to legacy specifications or admit pending change specifications.
Molten traceability maintainers own these readers and their filesystem-boundary tests.

Validation of this repair:

- Both original test suites passed: four tests each.
- Each suite then reproduced two failures with the new filesystem cases.
- Both final suites passed: six tests each.
- The cases cover native definitions, excluded legacy and pending definitions, missing directories, and non-directory roots.
- Negative cases assert the exact root-admission error.
- Both standalone test targets passed Clippy with `-D warnings`.
- Rust formatting and whitespace checks passed.
- Final strict Cairn validation and the proposal, design, and tasks gates passed. These results cover structural policy, not implementation acceptance.
- The classifier generated a scratch report from the existing reviewed baseline.

The corrected guard and the focused Nix check still deny the current source tree:

```text
requirements=2777
referenced=787
uncovered=1990
baseline_entries=1924
dangling=8
```

The dangling references are four `aspen.cas.*` identifiers and the four pending `molten.audit_f12.*` identifiers.
The uncovered count also differs from the reviewed baseline count.
The changes preserve the reviewed baseline, stored classification reports, accepted specifications, and gate thresholds.
The scratch classification is not an accepted replacement for the stored report.

This repair makes the reader use the current source location. It does not establish coverage.
The full Nix run preceded the reader repair; the focused Nix failure records the repaired reader.
A passing whole-repository Nix result is not claimed.

## Completion boundary

Strict source acceptance, Tracey acceptance, and all-feature Cargo metadata remain blocked.
The change is not synced, archived, or integrated. The target remains `origin/molten`, not legacy `main`.
The primary checkout and its unrelated work remain outside this worktree's commits.

Run `b3sum --check digests.blake3` from this evidence directory to check the stored report and attachments.
Receipt references identify canonical runtime values. The digest manifest identifies the stored file bytes.
