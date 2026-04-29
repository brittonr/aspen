## Why

Review metrics show repeated human/oracle-route findings where supplied evidence is insufficient to decide design/task consistency. The workflow needs an explicit escalation checkpoint instead of allowing ambiguous evidence to pass or be overclaimed.

## What Changes

- Add an oracle checkpoint mechanism for OpenSpec gates and done-review when evidence is incomplete or ambiguous.
- Require the checkpoint to record the unresolved question, evidence inspected, and decision outcome.
- Allow agents to stop with a concrete blocker instead of claiming completion.

## Capabilities

### New Capabilities
- `openspec-governance.review-oracle-checkpoints`: Ambiguous review states are escalated and recorded instead of silently passing.

## Impact

- **Files**: review/gate guidance, review metrics promotion handling, optional checkpoint template.
- **Testing**: Positive checkpoint record; negative ambiguous completion without checkpoint.
