# OpenSpec Preflight Evidence Fixtures

`scripts/test-openspec-preflight-evidence.sh` materializes these fixtures in a temporary git repository so each case can control tracked, staged, and untracked state without polluting this checkout.

## Positive cases

- `active-change-local`: active change cites a tracked evidence file under its own `evidence/` directory.
- `active-changed-file`: active change cites a currently changed documentation/source file listed in `## Implementation Evidence`.
- `archived-change-local`: archived change path cites a tracked evidence file under its own `evidence/` directory.
- `archived-changed-file`: archived change path cites a currently changed documentation/source file listed in `## Implementation Evidence`.

## Negative cases

Each negative case runs in both active (`openspec/changes/<case>`) and archived (`openspec/changes/archive/2026-04-29-<case>`) layouts.

- `missing-evidence`: checked task has no `- Evidence:` line.
- `untracked-evidence`: checked task cites an existing but untracked evidence file.
- `empty-evidence`: checked task cites an empty tracked evidence file.
- `pending-evidence`: checked task cites a tracked evidence file containing only `pending`.
- `todo-evidence`: checked task cites a tracked evidence file containing only `TODO`.
- `placeholder-evidence`: checked task cites a tracked evidence file containing only `placeholder`.
- `disallowed-external`: checked task cites a tracked file outside the change that is not listed as implementation evidence.

Set `OPENSPEC_PREFLIGHT_SHOW_CASE_OUTPUT=1` to include full per-case command, stdout, stderr, expected status, and actual status in the saved transcript.
