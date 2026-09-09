# F09 blocked runnable resume

## ADDED Requirements

### Requirement: Blocked Wake resumes the same occurrence

r[molten.audit_f09.resume]
Molten MUST admit current-generation Wake for an existing Blocked occurrence without another runnable record or active-slot charge.

#### Scenario: Full active capacity still permits resume
- GIVEN one blocked occurrence occupies the admitted active limit and the ready queue has space
- WHEN Wake names that occurrence
- THEN the phase becomes Ready and the active count stays unchanged

#### Scenario: Another phase remains a duplicate
- GIVEN the occurrence is Ready, Running, Completed, or Cancelled
- WHEN Wake names that occurrence
- THEN duplicate admission denies without state mutation

### Requirement: Resume shares bounded ready admission

r[molten.audit_f09.queue]
Molten MUST apply `max_scheduler_queue_depth` and the admitted overload policy before blocked resume or new Wake enters Ready.

#### Scenario: Queue capacity exists
- GIVEN a blocked occurrence and a free ready slot
- WHEN Wake passes admission
- THEN exactly one ready entry represents the occurrence

#### Scenario: Queue capacity is exhausted
- GIVEN the ready count equals `max_scheduler_queue_depth`
- WHEN Wake requests blocked resume
- THEN Reject or Backpressure preserves phases, counts, priorities, and sequence positions

### Requirement: Resume preserves explicit ordering and fences

r[molten.audit_f09.ordering]
Molten MUST assign a checked fresh enqueue sequence, apply the requested priority, and reset ready wait accounting after successful resume.
Molten MUST preserve state on arithmetic error and discard stale-generation commands without effects.

#### Scenario: Resume enters FIFO order
- GIVEN older ready work and a blocked current occurrence
- WHEN Wake resumes that occurrence
- THEN its new enqueue position follows older ready work under FIFO policy

#### Scenario: A sequence cannot advance
- GIVEN the enqueue sequence is exhausted
- WHEN Wake requests resume
- THEN an overflow result preserves all state and prevents wake effects

### Requirement: Adapter and evidence claims remain scoped

r[molten.audit_f09.validation]
Molten MUST cover core and shell resume behavior with positive and negative normal repository tests.
Molten MUST distinguish executed F09 evidence from static shell analysis and MUST NOT claim global liveness or measured performance.

#### Scenario: The extension resumes at its active limit
- GIVEN matching service and profile bindings with queue space and a full active envelope
- WHEN the extension submits Wake for its blocked occurrence
- THEN adapter conformance records the admitted resume without another active charge

#### Scenario: A stale callback requests resume
- GIVEN a callback belongs to a retired generation
- WHEN its Wake reaches the adapter
- THEN no current-generation wake effect occurs and evidence records the scoped denial or discard
