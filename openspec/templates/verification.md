# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.
Do not rely on chat-only summaries, `/tmp` logs, or memory.

## Implementation Evidence

List the files changed for the change under review.
Each path must be repo-relative and currently appear in `git status`.

- Changed file: `path/to/file.rs`
- Changed file: `path/to/other_file.md`

## Task Coverage

Copy each checked task from `tasks.md` exactly and cite the evidence paths that justify it.
Every checked task must appear here.
Evidence paths must be tracked, repo-relative files that are non-empty and not placeholder-only (`pending`, `TODO`, or `placeholder`).
Task evidence may cite only files under the current change directory or currently modified/staged source/doc files listed above as changed files.
For implementation-complete claims, prefer citing a saved diff artifact in addition to the source paths.

- [x] Exact checked task text from tasks.md
  - Evidence: `path/to/changed_file.rs`, `openspec/changes/active/<change>/evidence/example.txt`

## Oracle Checkpoints

Use an oracle checkpoint when a reviewer or gate cannot resolve a review-critical
question from deterministic repo evidence. Store the checkpoint under the change
`evidence/` directory, or inline here, using `openspec/templates/oracle-checkpoint.md`.
A valid checkpoint records the unresolved question, inspected evidence, decision,
owner, and next action. If no decision exists, report a concrete blocker and stop
without claiming completion instead of fabricating certainty.

## Review Scope Snapshot

If a reviewer needs the exact implementation delta, save a diff artifact that covers the
files you are claiming as complete.

### `git diff HEAD -- path/to/file.rs path/to/tasks.md`

- Status: captured
- Artifact: `openspec/changes/active/<change>/evidence/implementation-diff.txt`

## Verification Commands

Record the commands actually run plus the durable artifacts that capture their output.
Keep artifacts inside the change directory, usually under `evidence/`.
If you claim preflight or syntax-check validation, save those transcripts too.

### `cargo test -p example-crate`

- Status: pass
- Artifact: `openspec/changes/active/<change>/evidence/cargo-test-example.txt`

### `nix run .#example -- --flag`

- Status: pass
- Artifact: `openspec/changes/active/<change>/evidence/example-run.txt`

### `bash -n scripts/openspec-preflight.sh`

- Status: pass
- Artifact: `openspec/changes/active/<change>/evidence/bash-n-openspec-preflight.txt`

### `scripts/openspec-preflight.sh <change-dir-or-name>`

- Status: pass
- Artifact: `openspec/changes/active/<change>/evidence/openspec-preflight.txt`

## Notes

- Run `scripts/openspec-preflight.sh <change-dir-or-name>` before checking the final task box or requesting done review.
- The preflight script now fails if the repo still contains untracked files unless `OPENSPEC_PREFLIGHT_ALLOW_UNTRACKED=1` is set.
- Stage newly created source files before `nix build` / `nix run`; untracked files can be excluded by the flake source filter.
