# Implementation review: exercise a service recovery composition end to end

## Disposition

Both tracks pass: the deterministic scenario matrix and the real-process twin against the signed
producer output with correct and broken cohorts. Evidence:
`evidence/2026-10-31-live-recovery-twin.log` and the workspace suite results in
`host-node-recovery-groups/evidence/`.

## Reviewed invariants

- The three-service composition is defined once through typed facades and consumed by both tracks.
- The five properties hold for the correct composition and fail for each broken variant: fencing
  removed, group budget reset and lost history.
- The live twin maps its observations to the same property vocabulary; timing checks use admitted
  observations, not rendered output.

## Non-claims

The composition exercises hosting and recovery behavior on one host; it does not prove whole
system correctness and does not patch the mechanism under test.
