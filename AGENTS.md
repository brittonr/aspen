# Agent Notes

## Repository and lifecycle

This checkout contains Molten. Read `README.md` and the relevant architecture documentation before changing product behavior.
Current native Cairn changes and specs live under `.cairn/`. Retain `cairn/archive/` as historical evidence.
Generated OpenSpec skills are legacy compatibility guidance, not the lifecycle authority. Preserve all required Cairn gates.

## Molten mainline exception

The repository is `brittonr/aspen`. Its mainline is `molten`, not the legacy `main` branch.
The SSH remote is `git@github.com:brittonr/aspen.git`.

For this repository, the Cairn completion workflow uses `origin/molten` wherever the general workflow names `origin/main`.
The local integration branch is `molten` instead of `main`.

- Create dedicated implementation worktrees from current `origin/molten`.
- Fetch `origin` again before integration.
- If `origin/molten` advanced, merge it into the change branch.
- After that merge, rerun the relevant checks.
- Integrate the verified change branch into `molten` by fast-forward.
- Verify that `origin/molten` contains the completion commit before worktree removal.

This exception changes branch names only. Preserve all required implementation, validation, review, sync, archive, commit, and push steps.
Keep task-specific publication authority, gate strength, and unrelated work intact.
Do not merge or overwrite the legacy `main` history. Do not force-push. Do not create pull requests.
This repository guidance does not override higher-priority session instructions.

## Generated agent guidance

Keep local workflow overrides here. Update shared templates through their owning generator.
For an explicitly selected, approved legacy workflow, apply these repository rules:

- Diagnose recoverable errors within the approved scope. Correct them before continuing.
- If a requirement or design decision changes the outcome, resolve it before implementation.
- If a lifecycle review or approval is required, complete it before the protected action.
- If a concrete blocker prevents progress, report its evidence and the next required decision.
- Preserve read-only exploration, ambiguous change selection, and all required evidence or archive gates.

## Historical debug prompts

The ignored `.claude/prompts/` files describe earlier Aspen CI/VM workflows.
Before using one, verify that its script and target components exist in the selected checkout.
If they are absent, report the obsolete procedure. Do not recreate removed components merely to satisfy the prompt.

Keep retries bounded and tied to a relevant fix or new evidence. Stop only task-owned processes and remove only task-owned scratch paths.
Preserve unrelated changes and failed-attempt evidence. Report passed, blocked, or budget-exhausted results without claiming unrun tests.
