## Why

Done-review repeatedly flags final responses that claim clean working trees, completed queues, or successful checks without supplied evidence. Completion summaries should be constrained by captured facts.

## What Changes

- Add a completion evidence checklist for agents before final summaries.
- Require saved or recent command output for claims about git status, active OpenSpec queue, and verification commands.
- Add a done-review deterministic check that flags unsupported completion claims.

## Capabilities

### New Capabilities
- `openspec-governance.completion-claim-evidence`: Completion claims are backed by explicit command evidence.

## Impact

- **Files**: done-review extension, prompt guidance, optional completion checklist template.
- **Testing**: Positive summary with evidence and negative summary with unsupported clean-status claim.
