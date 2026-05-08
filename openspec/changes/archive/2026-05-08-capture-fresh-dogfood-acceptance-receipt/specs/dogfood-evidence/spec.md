## ADDED Requirements

### Requirement: Fresh Dogfood Acceptance Receipt [r[dogfood-evidence.fresh-acceptance-receipt]]

Aspen MUST treat a fresh dogfood full-loop acceptance claim as valid only when a current-HEAD dogfood run produces a durable, secret-safe receipt and operator readback evidence.

#### Scenario: Current HEAD full dogfood succeeds [r[dogfood-evidence.fresh-acceptance-receipt.current-head-success]]

- GIVEN the Aspen checkout is clean and `HEAD` is the intended source revision
- WHEN `nix run .#dogfood-local -- full` completes successfully
- THEN a dogfood run receipt SHALL identify the run, command, source context, ordered stages, final success status, and relevant artifact references
- AND the evidence SHALL be inspectable without relying on chat-only logs

#### Scenario: Receipt readback validates acceptance [r[dogfood-evidence.fresh-acceptance-receipt.readback]]

- GIVEN a successful dogfood run receipt exists
- WHEN an operator uses receipt list, show, or diagnose commands against the configured receipt store
- THEN the commands SHALL surface the accepted final status, stage summary, elapsed timing where available, and artifact references without requiring a running cluster

#### Scenario: Failed dogfood run is not acceptance [r[dogfood-evidence.fresh-acceptance-receipt.failure-boundary]]

- GIVEN a dogfood full run exits unsuccessfully or records a failed stage
- WHEN evidence is captured for the run
- THEN Aspen SHALL record diagnostic evidence and failure category without marking the run accepted
- AND the OpenSpec implementation tasks SHALL remain incomplete until a successful rerun or explicit scope change exists
