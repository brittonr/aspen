# Proposal gate worktree audit

- Status: local artifact fixed; `openspec_gate` report is stale for this `.pi/worktrees` checkout because it reads the sibling main checkout.
- Gate stale finding: reported `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/specs/core/spec.md` still starts with `## MODIFIED Requirements`.

## Local worktree evidence

```text
pwd=/home/brittonr/git/aspen/.pi/worktrees/session-1777140788576-d2t2
git_root=/home/brittonr/git/aspen/.pi/worktrees/session-1777140788576-d2t2
worktree_core_delta_header=## ADDED Requirements
main_checkout_core_delta_header=## MODIFIED Requirements

Change 'extend-no-std-foundation-and-wire' is valid
OK: openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire
  tasks: 13 total / 13 checked
  changed files listed: 4
  verification artifacts: 1
```

## Decision

Proceed using local `openspec validate` plus `scripts/openspec-preflight.sh` as the next-best checks for this worktree, matching the known gate-tool worktree visibility issue.
