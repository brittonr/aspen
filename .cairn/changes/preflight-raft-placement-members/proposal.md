# Preflight admitted Raft placement members

## Why

`consensus_placement_report` accepts caller-built `ConsensusPlacementInput`.
It validates `admitted_members` as generic refs, allowing 4,096 entries,
then formats one diagnostic for every admitted member absent from the
candidate pool and serializes/clones the entire list. The membership owner
already defines `MAX_RAFT_MEMBERS = 32` and diagnostics max 256; this public
producer bypasses the smaller member boundary.

## What Changes

- Admit at most 32 actual `admitted_members` before validating generic
  refs, formatting placement diagnostics or cloning them into a report.
- Preserve existing valid 32-member placement, canonical-ref errors and
  duplicate-member behavior for admitted inputs.
- Keep `candidate_members` as the previously bounded generic candidate
  pool (up to `MAX_RAFT_REFS = 4,096`), not newly subject to the
  membership cap; candidate eligibility is not actual membership.

## Impact

One canonical public producer, focused unit regression, compiled external
`molten::raft_control_plane` consumer, handoff, and one change-local Cairn
delta. The detached composition receives disjoint equivalent source,
tests, docs and evidence atop 65 existing packages. The production CLI
only parses placement reports; no producer CLI path is claimed. No
policy, waiver, baseline, pin, accepted spec or unchanged full strict-ci
retry is added.
