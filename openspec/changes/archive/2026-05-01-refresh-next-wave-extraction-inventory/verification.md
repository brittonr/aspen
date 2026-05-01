# Verification Evidence

## Implementation Evidence

- Changed file: `docs/crate-extraction.md`
- Changed file: `openspec/specs/architecture-modularity/spec.md`
- Changed file: `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/.openspec.yaml`
- Changed file: `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/proposal.md`
- Changed file: `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/design.md`
- Changed file: `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/tasks.md`
- Changed file: `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/verification.md`
- Changed file: `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/specs/architecture-modularity/spec.md`
- Changed file: `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/openspec-validate.json`
- Changed file: `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/helper-verify.json`
- Changed file: `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/diff-check.txt`
- Changed file: `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/openspec-preflight.txt`
- Changed file: `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/openspec-drain-audit.txt`

## Task Coverage

- [x] I1 Refresh next-wave broader inventory rows to link existing manifests, owner groups, and current evidence-backed next actions. [covers=architecture-modularity.extraction-inventory-tracks-next-wave-evidence.manifest-links,architecture-modularity.extraction-inventory-tracks-next-wave-evidence.current-next-actions] ✅ 5m (started: 2026-05-01T20:35:27Z → completed: 2026-05-01T20:40:27Z)
  - Evidence: `docs/crate-extraction.md`, `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/openspec-validate.json`
- [x] V1 Run OpenSpec validation, helper verification, preflight, and whitespace checks for the inventory refresh. [covers=architecture-modularity.extraction-inventory-tracks-next-wave-evidence.manifest-links,architecture-modularity.extraction-inventory-tracks-next-wave-evidence.current-next-actions] ✅ 5m (started: 2026-05-01T20:40:27Z → completed: 2026-05-01T20:45:27Z)
  - Evidence: `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/helper-verify.json`, `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/diff-check.txt`, `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/openspec-preflight.txt`

## Drain Verification Matrix

| Rail | Command | Status | Artifact | Scope rationale | Next best check |
| --- | --- | --- | --- | --- | --- |
| build | `openspec validate refresh-next-wave-extraction-inventory --json` | pass | `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/openspec-validate.json` | Validates the OpenSpec package and delta spec for this docs/governance slice. | Validate the merged `architecture-modularity` domain after archive. |
| test | `python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify refresh-next-wave-extraction-inventory --json` | pass | `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/helper-verify.json` | Confirms tasks and delta-spec mechanics for the active change. | Archive helper flow after tasks remain complete. |
| format | `git diff --check` | pass | `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/diff-check.txt` | This is a docs/OpenSpec-only slice; whitespace is the relevant formatting rail. | `git diff --cached --check` before commit. |
| preflight | `scripts/openspec-preflight.sh refresh-next-wave-extraction-inventory` | pass | `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/openspec-preflight.txt` | Ensures checked tasks have durable evidence and changed-file entries. | Run again after staging final evidence if metadata changes. |

## Verification Commands

### `openspec validate refresh-next-wave-extraction-inventory --json`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/openspec-validate.json`

### `python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify refresh-next-wave-extraction-inventory --json`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/helper-verify.json`

### `git diff --check`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/diff-check.txt`

### `scripts/openspec-preflight.sh refresh-next-wave-extraction-inventory`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/openspec-preflight.txt`

### `scripts/openspec-drain-audit.sh --archive openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-refresh-next-wave-extraction-inventory/evidence/openspec-drain-audit.txt`
